use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use futures::stream::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use serde::Deserialize;
use tokio::time::sleep;

mod config;
mod db;

#[derive(Debug, Deserialize)]
struct HelloMessage {
    name: String,
    timestamp: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        log::info!("Received shutdown signal");
        r.store(false, Ordering::SeqCst);
    })?;

    log::info!("Initializing consumer...");

    let config_str = std::fs::read_to_string("config.json")?;
    let config = config::ConsumerConfig::new(&config_str)?;
    
    // Initialize database connection pool
    let db_pool = db::create_pool(&config.database).await?;
    // Verify database connection and schema
    let conn = db_pool.get().await?;
    conn.execute("SELECT 1", &[]).await?;
    
    // Check if messages table exists
    let table_exists = conn
        .query_one(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'messages'
            )",
            &[],
        )
        .await?
        .get::<_, bool>(0);
    
    if !table_exists {
        log::error!("Messages table does not exist in database. Please ensure init.sql was properly executed.");
        log::error!("Current database: {}", config.database.dbname);
        return Err("Database schema not initialized".into());
    }
    
    log::info!("Successfully connected to PostgreSQL database and verified schema");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_broker)
        .set("group.id", &config.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&[&config.topic])?;
    log::info!("Listening to topic: {}", config.topic);

    let mut message_stream = consumer.stream();

    // process messages or check shutdown flag.
    while running.load(Ordering::SeqCst) {
        tokio::select! {
            maybe_msg = message_stream.next() => {
                if let Some(result) = maybe_msg {
                    handle_message(result, &db_pool).await;
                }
            },
            _ = sleep(Duration::from_millis(100)) => {
                // Allows periodic re-check of the shutdown flag.
            }
        }
    }

    log::info!("Shutting down consumer...");
    drop(message_stream);
    drop(consumer);
    sleep(Duration::from_secs(1)).await;
    Ok(())
}

async fn handle_message(
    result: rdkafka::error::KafkaResult<rdkafka::message::BorrowedMessage<'_>>,
    db_pool: &db::Pool,
) {
    match result {
        Ok(message) => {
            if let Some(payload) = message.payload() {
                let payload_str = String::from_utf8_lossy(payload);
                log::info!("Received message: {}", payload_str);
                
                // Store in PostgreSQL with retries
                let max_retries = 3;
                let mut retry_count = 0;
                let mut last_error = None;
                
                while retry_count < max_retries {
                    match db::insert_message(
                        db_pool,
                        message.topic(),
                        message.partition(),
                        message.offset(),
                        &payload_str
                    ).await {
                        Ok(_) => {
                            log::info!(
                                "Successfully stored message in database - Topic: {}, Partition: {}, Offset: {}", 
                                message.topic(), 
                                message.partition(), 
                                message.offset()
                            );
                            // Process message content only after successful DB write
                            if let Err(e) = process_message(&payload_str) {
                                log::error!("Failed to process message: {}", e);
                            }
                            return;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            retry_count += 1;
                            log::warn!(
                                "Failed to store message in database (attempt {}/{}): {}", 
                                retry_count, 
                                max_retries, 
                                last_error.as_ref().unwrap()
                            );
                            if retry_count < max_retries {
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
                
                if let Some(e) = last_error {
                    log::error!(
                        "Failed to store message in database after {} retries. Final error: {}", 
                        max_retries, 
                        e
                    );
                }
            }
        }
        Err(e) => {
            log::error!("Error receiving message: {}", e);
        }
    }
}

/// Processes the payload by attempting to deserialize it as JSON.
/// If JSON parsing fails, it handles the payload as plain text.
fn process_message(payload: &str) -> Result<(), Box<dyn Error>> {
    if let Ok(message) = serde_json::from_str::<HelloMessage>(payload) {
        log::info!(
            "Processed JSON greeting for {} (timestamp: {})",
            message.name,
            message.timestamp
        );
    } else {
        log::info!("Processed plain text message: {}", payload);
    }
    Ok(())
}
