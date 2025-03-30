use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::stream::StreamExt;
use prost::Message as ProstMessage;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message as KafkaMessage;
use serde::Deserialize;
use tokio::time::sleep;

mod config;
mod db;

use common_proto::envelope::Envelope;
use common_proto::proto::PublishRequest;

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

    // Create a consumer for the default topic
    let default_consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_broker)
        .set("group.id", &config.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;
    default_consumer.subscribe(&[&config.topic])?;
    log::info!("Listening to default topic: {}", config.topic);

    // Create a consumer for the publish topic
    let publish_consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_broker)
        .set("group.id", format!("{}-publish", config.group_id))
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;
    publish_consumer.subscribe(&[&config.publish_topic])?;
    log::info!("Listening to publish topic: {}", config.publish_topic);

    let mut default_message_stream = default_consumer.stream();
    let mut publish_message_stream = publish_consumer.stream();

    while running.load(Ordering::SeqCst) {
        tokio::select! {
            maybe_msg = default_message_stream.next() => {
                if let Some(result) = maybe_msg {
                    process_message(result, &db_pool).await;
                }
            },
            maybe_msg = publish_message_stream.next() => {
                if let Some(result) = maybe_msg {
                    process_publish_message(result, &db_pool).await;
                }
            },
            _ = sleep(Duration::from_millis(100)) => {}
        }
    }

    log::info!("Shutting down consumer...");
    drop(default_message_stream);
    drop(publish_message_stream);
    drop(default_consumer);
    drop(publish_consumer);
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

async fn process_publish_message(
    result: rdkafka::error::KafkaResult<rdkafka::message::BorrowedMessage<'_>>,
    db_pool: &db::Pool,
) {
    let message = match result {
        Ok(msg) => msg,
        Err(e) => {
            log::error!("Error receiving publish message: {}", e);
            return;
        }
    };

    let payload = match message.payload() {
        Some(p) => p,
        None => {
            log::error!("Received empty publish message payload");
            return;
        }
    };

    log::info!(
        "Received publish message - Topic: {}, Partition: {}, Offset: {}",
        message.topic(),
        message.partition(),
        message.offset()
    );

    // Try to decode the envelope
    let envelope = match Envelope::decode(bytes::Bytes::copy_from_slice(payload)) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to decode envelope: {}", e);
            return;
        }
    };

    log::info!(
        "Decoded envelope - Type: {}, Timestamp: {}, Version: {}",
        envelope.message_type,
        envelope.timestamp,
        envelope.version
    );

    // Verify the message type
    if envelope.message_type != "hello.PublishRequest" {
        log::warn!(
            "Unexpected message type: {}, expected hello.PublishRequest",
            envelope.message_type
        );
        return;
    }

    // Try to decode the PublishRequest from the envelope payload
    let publish_request = match PublishRequest::decode(bytes::Bytes::copy_from_slice(&envelope.payload)) {
        Ok(req) => req,
        Err(e) => {
            log::error!("Failed to decode PublishRequest: {}", e);
            return;
        }
    };

    log::info!(
        "Decoded PublishRequest - Data: {}, Metadata: {}",
        publish_request.data,
        publish_request.metadata
    );

    // Insert the published data into the database
    const MAX_RETRIES: usize = 3;
    let mut inserted = false;
    for attempt in 1..=MAX_RETRIES {
        let metadata = if publish_request.metadata.is_empty() {
            None
        } else {
            Some(publish_request.metadata.as_str())
        };

        if db::insert_published_data(
            db_pool,
            message.topic(),
            message.partition(),
            message.offset(),
            &publish_request.data,
            metadata,
        )
        .await
        .is_ok()
        {
            log::info!(
                "Successfully stored published data in database - Topic: {}, Partition: {}, Offset: {}",
                message.topic(),
                message.partition(),
                message.offset()
            );
            inserted = true;
            break;
        } else {
            log::warn!(
                "Failed to store published data in database (attempt {}/{})",
                attempt,
                MAX_RETRIES
            );
            if attempt < MAX_RETRIES {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    if !inserted {
        log::error!("Failed to store published data in database after {} attempts", MAX_RETRIES);
    }
}
