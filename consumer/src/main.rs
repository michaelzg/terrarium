use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::Message;
use std::env;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use futures::stream::StreamExt;
use serde::Deserialize;

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
    loop {
        tokio::select! {
            Some(message_result) = message_stream.next() => {
                match message_result {
                    Ok(message) => {
                        if let Some(payload) = message.payload() {
                            let payload_str = String::from_utf8_lossy(payload);
                            log::info!("Received message: {}", payload_str);

                            if let Err(e) = process_message(&payload_str) {
                                log::error!("Failed to process message: {}", e);
                                // Simple retry mechanism: retry once
                                if let Err(e) = process_message(&payload_str) {
                                    log::error!("Failed to process message after retry: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error receiving message: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if !running.load(Ordering::SeqCst) {
                    log::info!("Shutting down consumer...");
                    drop(message_stream);
                    drop(consumer);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    return Ok(());
                }
            }
        }
    }
}

fn process_message(payload: &str) -> Result<(), Box<dyn Error>> {
    // Try to parse as JSON first
    match serde_json::from_str::<HelloMessage>(payload) {
        Ok(message) => {
            // Process structured JSON message
            log::info!(
                "Processed JSON greeting for {} (timestamp: {})", 
                message.name, 
                message.timestamp
            );
        }
        Err(_) => {
            // Handle as plain text if JSON parsing fails
            log::info!("Processed plain text message: {}", payload);
        }
    }
    
    Ok(())
}
