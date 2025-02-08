use std::env;
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

    let config_str = env::var("CONSUMER_CONFIG")?;
    let config = config::ConsumerConfig::new(&config_str)?;

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
                    handle_message(result);
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

fn handle_message(result: rdkafka::error::KafkaResult<rdkafka::message::BorrowedMessage>) {
    match result {
        Ok(message) => {
            if let Some(payload) = message.payload() {
                let payload_str = String::from_utf8_lossy(payload);
                log::info!("Received message: {}", payload_str);
                if let Err(e) = process_message(&payload_str) {
                    log::error!("Failed to process message: {}. Retrying...", e);
                    if let Err(e) = process_message(&payload_str) {
                        log::error!("Retry failed: {}", e);
                    }
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