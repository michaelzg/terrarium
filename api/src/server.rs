use common_proto::proto::hello_api_server::{HelloApi, HelloApiServer};
use common_proto::proto::{GetMessagesReply, GetMessagesRequest, HelloReply, HelloRequest};
use log::{error, info};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::fs;
use chrono::{DateTime, Utc};
use hyper::{Body, Request as HttpRequest, Response as HttpResponse, Server as HttpServer};
use hyper::service::{make_service_fn, service_fn};
use prometheus::{Encoder, IntCounter, Histogram, HistogramOpts, Opts, Registry, TextEncoder};
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    kafka_broker: String,
    topic: String,
    database: DatabaseSettings,
}

#[derive(Debug, Deserialize)]
struct DatabaseSettings {
    host: String,
    port: u16,
    user: String,
    password: String,
    dbname: String,
    pool_size: usize,
}

impl ServerConfig {
    fn load(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string(config_path)?;
        let config: Self = serde_json::from_str(&config_str)?;
        info!("ServerConfig loaded successfully");
        Ok(config)
    }
}

impl DatabaseSettings {
    fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.dbname
        )
    }
}

// Use proto module from common_proto crate
use common_proto::proto;

#[derive(Debug, Serialize)]
struct HelloEvent {
    name: String,
    produced_at: DateTime<Utc>,
}

pub struct KafkaService {
    kafka_producer: FutureProducer,
    topic: String,
    messages_published: IntCounter,
}

impl KafkaService {
    pub fn new(config: &ServerConfig, registry: &Registry) -> KafkaService {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.kafka_broker)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        let messages_published = IntCounter::with_opts(Opts::new(
            "api_messages_published_total",
            "Total number of messages published to Kafka by the API",
        ))
        .expect("failed to create api_messages_published_total metric");

        registry
            .register(Box::new(messages_published.clone()))
            .expect("failed to register api_messages_published_total metric");

        KafkaService {
            kafka_producer: producer,
            topic: config.topic.clone(),
            messages_published,
        }
    }

    async fn publish(&self, name: &String) -> Result<(), Status> {
        let event = HelloEvent {
            name: name.clone(),
            produced_at: Utc::now(),
        };
        let payload = serde_json::to_string(&event).map_err(|e| {
            Status::internal(format!("Failed to serialize event payload: {}", e))
        })?;

        let record = FutureRecord::to(&self.topic).key(name).payload(&payload);

        self.kafka_producer
            .send(record, std::time::Duration::from_secs(5))
            .await
            .map_err(|err| Status::internal(format!("Failed to send message: {:?}", err)))?;

        self.messages_published.inc();
        info!("Published HelloEvent for {}", name);
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct DbMessage {
    id: i32,
    topic: String,
    part: i32,
    kafkaoffset: i64,
    payload: String,
    created_at: String,
}

pub struct MyHelloApi {
    kafka: KafkaService,
    db_pool: Pool<Postgres>,
}

impl MyHelloApi {
    async fn new(config: &ServerConfig, registry: &Registry) -> Result<Self, sqlx::Error> {
        let kafka = KafkaService::new(config, registry);
        let pool = PgPoolOptions::new()
            .max_connections(config.database.pool_size as u32)
            .connect(&config.database.connection_string())
            .await?;

        Ok(Self {
            kafka,
            db_pool: pool,
        })
    }

    async fn get_messages_from_db(
        &self,
        topic: &str,
        limit: i32,
    ) -> Result<Vec<proto::Message>, sqlx::Error> {
        let messages = sqlx::query_as::<_, DbMessage>(
            "SELECT id, topic, part, kafkaoffset, payload, created_at::text as created_at \
             FROM messages \
             WHERE topic = $1 \
             ORDER BY created_at DESC \
             LIMIT $2",
        )
        .bind(topic)
        .bind(limit as i64)
        .fetch_all(&self.db_pool)
        .await?;

        Ok(messages
            .into_iter()
            .map(|m| proto::Message {
                id: m.id,
                topic: m.topic,
                part: m.part,
                kafkaoffset: m.kafkaoffset,
                payload: m.payload,
                created_at: m.created_at,
            })
            .collect())
    }
}

#[tonic::async_trait]
impl HelloApi for MyHelloApi {
    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        let req = request.into_inner();
        let messages = self
            .get_messages_from_db(&req.topic, req.limit)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(GetMessagesReply { messages }))
    }

    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Received a request: {:?}", request);
        let name = request.into_inner().name;

        self.kafka
            .publish(&name)
            .await
            .map_err(|err| Status::internal(format!("Failed to send message: {:?}", err)))?;

        let reply = proto::HelloReply {
            message: format!("Hello {}!", name),
        };

        Ok(Response::new(reply))
    }
}

async fn handle_http(
    req: HttpRequest<Body>,
    registry: Registry,
) -> Result<HttpResponse<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/metrics") => {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                error!("Failed to encode metrics: {}", e);
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
            let body = "<html><head><title>API Dashboard</title></head><body><h1>API Dashboard</h1><p>Prometheus metrics are available at <a href=\"/metrics\">/metrics</a>.</p><p>Health check: <a href=\"/healthz\">/healthz</a></p></body></html>";
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = ServerConfig::load("api/config.json")?;
    let addr = "127.0.0.1:50051".parse()?;

    // Metrics registry for the API
    let registry = Registry::new();

    // Spawn HTTP server for metrics and dashboard
    let http_registry = registry.clone();
    tokio::spawn(async move {
        let make_svc = make_service_fn(move |_| {
            let registry = http_registry.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: HttpRequest<Body>| {
                    let registry = registry.clone();
                    async move { handle_http(req, registry).await }
                }))
            }
        });

        let addr = ([0, 0, 0, 0], 9000).into();
        info!("Starting API metrics HTTP server on {}", addr);

        if let Err(e) = HttpServer::bind(&addr).serve(make_svc).await {
            error!("API HTTP server error: {}", e);
        }
    });

    let api = MyHelloApi::new(&config, &registry)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()?;

    info!("Server is online at {}", addr);

    Server::builder()
        .add_service(reflection)
        .add_service(HelloApiServer::new(api))
        .serve(addr)
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

    Ok(())
}
