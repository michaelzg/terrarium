use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

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

    let db_pool = db::create_pool(&config.database).await?;

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

    while running.load(Ordering::SeqCst) {
        tokio::select! {
            maybe_msg = message_stream.next() => {
                if let Some(result) = maybe_msg {
                    process_message(result, &db_pool).await;
                }
            },
            _ = sleep(Duration::from_millis(100)) => {}
        }
    }

    log::info!("Shutting down consumer...");
    drop(message_stream);
    drop(consumer);
    sleep(Duration::from_secs(1)).await;
    Ok(())
}

async fn process_message(
    result: rdkafka::error::KafkaResult<rdkafka::message::BorrowedMessage<'_>>,
    db_pool: &db::Pool,
) {
    let message = match result {
        Ok(msg) => msg,
        Err(e) => {
            log::error!("Error receiving message: {}", e);
            return;
        }
    };

    let payload = match message.payload() {
        Some(p) => p,
        None => return,
    };

    let payload_str = String::from_utf8_lossy(payload);
    log::info!("Received message: {}", payload_str);

    const MAX_RETRIES: usize = 3;
    let mut inserted = false;
    for attempt in 1..=MAX_RETRIES {
        if db::insert_message(
            db_pool,
            message.topic(),
            message.partition(),
            message.offset(),
            &payload_str,
        )
        .await
        .is_ok()
        {
            log::info!(
                "Successfully stored message in database - Topic: {}, Partition: {}, Offset: {}",
                message.topic(),
                message.partition(),
                message.offset()
            );
            inserted = true;
            break;
        } else {
            log::warn!(
                "Failed to store message in database (attempt {}/{})",
                attempt,
                MAX_RETRIES
            );
            if attempt < MAX_RETRIES {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    if !inserted {
        log::error!("Failed to store message in database after {} attempts", MAX_RETRIES);
        return;
    }

    if let Ok(parsed) = serde_json::from_str::<HelloMessage>(&payload_str) {
        log::info!(
            "Processed JSON greeting for {} (timestamp: {})",
            parsed.name,
            parsed.timestamp
        );
    } else {
        log::info!("Processed plain text message: {}", payload_str);
    }
}