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
use chrono::{DateTime, Utc};
use hyper::{Body, Request as HttpRequest, Response as HttpResponse, Server as HttpServer};
use hyper::service::{make_service_fn, service_fn};
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, Opts, Registry, TextEncoder};

mod config;
mod db;

#[derive(Debug, Deserialize)]
struct HelloMessage {
    name: String,
    produced_at: DateTime<Utc>,
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

    // Metrics registry and exporters
    let registry = Registry::new();
    let messages_consumed = IntCounter::with_opts(Opts::new(
        "consumer_messages_total",
        "Total number of messages consumed from Kafka",
    ))?;
    let db_insert_failures = IntCounter::with_opts(Opts::new(
        "consumer_db_insert_failures_total",
        "Total number of DB insert failures",
    ))?;
    let end_to_end_latency = Histogram::with_opts(HistogramOpts::new(
        "consumer_end_to_end_latency_seconds",
        "End-to-end latency from API to DB sink",
    ))?;

    registry.register(Box::new(messages_consumed.clone()))?;
    registry.register(Box::new(db_insert_failures.clone()))?;
    registry.register(Box::new(end_to_end_latency.clone()))?;

    // Spawn HTTP server for metrics and a simple dashboard
    let http_registry = registry.clone();
    let http_running = running.clone();
    tokio::spawn(async move {
        let make_svc = make_service_fn(move |_| {
            let registry = http_registry.clone();
            let running = http_running.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: HttpRequest<Body>| {
                    let registry = registry.clone();
                    let running = running.clone();
                    async move { handle_http(req, registry, running).await }
                }))
            }
        });

        let addr = ([0, 0, 0, 0], 9100).into();
        log::info!("Starting consumer metrics HTTP server on {}", addr);

        if let Err(e) = HttpServer::bind(&addr).serve(make_svc).await {
            log::error!("HTTP server error: {}", e);
        }
    });

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
                    process_message(result, &db_pool, &messages_consumed, &db_insert_failures, &end_to_end_latency).await;
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
    messages_consumed: &IntCounter,
    db_insert_failures: &IntCounter,
    end_to_end_latency: &Histogram,
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

    messages_consumed.inc();

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
        db_insert_failures.inc();
        return;
    }

    if let Ok(parsed) = serde_json::from_str::<HelloMessage>(&payload_str) {
        let now = Utc::now();
        let latency = (now - parsed.produced_at).num_microseconds().unwrap_or(0) as f64 / 1_000_000.0;
        if latency >= 0.0 {
            end_to_end_latency.observe(latency);
        }
        log::info!(
            "Processed JSON greeting for {} (produced_at: {}, end_to_end_latency_s: {:.6})",
            parsed.name,
            parsed.produced_at,
            latency
        );
    } else {
        log::info!("Processed plain text message: {}", payload_str);
    }
}

async fn handle_http(
    req: HttpRequest<Body>,
    registry: Registry,
    _running: Arc<AtomicBool>,
) -> Result<HttpResponse<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/metrics") => {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                log::error!("Failed to encode metrics: {}", e);
                return Ok(HttpResponse::builder()
                    .status(500)
                    .body(Body::from("failed to encode metrics"))
                    .unwrap());
            }
            Ok(HttpResponse::builder()
                .status(200)
                .header("Content-Type", encoder.format_type())
                .body(Body::from(buffer))
                .unwrap())
        }
        (&hyper::Method::GET, "/healthz") => Ok(HttpResponse::new(Body::from("ok"))),
        (&hyper::Method::GET, "/dashboard") => {
            let body = "<html><head><title>Consumer Dashboard</title></head><body><h1>Consumer Dashboard</h1><p>Prometheus metrics are available at <a href=\"/metrics\">/metrics</a>.</p><p>Health check: <a href=\"/healthz\">/healthz</a></p></body></html>";
            Ok(HttpResponse::builder()
                .status(200)
                .header("Content-Type", "text/html; charset=utf-8")
                .body(Body::from(body))
                .unwrap())
        }
        _ => Ok(HttpResponse::builder()
            .status(404)
            .body(Body::from("not found"))
            .unwrap()),
    }
}
