use log::{error, info};
use proto::hello_api_server::{HelloApi, HelloApiServer};
use proto::{GetMessagesReply, GetMessagesRequest, HelloReply, HelloRequest};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::fs;
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    kafka_broker: String,
    topic: String,
    database: DatabaseSettings,
    grpc: GrpcSettings,
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

#[derive(Debug, Deserialize)]
struct GrpcSettings {
    host: String,
    port: u16,
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

mod proto {
    tonic::include_proto!("hello");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("hello_descriptor");
}

pub struct KafkaService {
    kafka_producer: FutureProducer,
    topic: String,
}

impl KafkaService {
    pub fn new(config: &ServerConfig) -> KafkaService {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.kafka_broker)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        KafkaService {
            kafka_producer: producer,
            topic: config.topic.clone(),
        }
    }

    async fn publish(&self, name: &String) -> Result<(), Status> {
        let payload = format!("Hello {}", name);
        let record = FutureRecord::to(&self.topic).key(name).payload(&payload);

        self.kafka_producer
            .send(record, std::time::Duration::from_secs(5))
            .await
            .map_err(|err| Status::internal(format!("Failed to send message: {:?}", err)))?;

        info!("Message published for {}", name);
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
    async fn new(config: &ServerConfig) -> Result<Self, sqlx::Error> {
        let kafka = KafkaService::new(config);
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
            "SELECT id, topic, part, kafkaoffset, payload, created_at::text as created_at 
             FROM messages 
             WHERE topic = $1 
             ORDER BY created_at DESC 
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = ServerConfig::load("config.json")?;
    let addr = format!("{}:{}", config.grpc.host, config.grpc.port).parse()?;
    let api = MyHelloApi::new(&config)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()?;

    info!("Server is online at {}", addr);

    Server::builder()
        .accept_http1(true)
        .add_service(tonic_web::enable(reflection))
        .add_service(tonic_web::enable(HelloApiServer::new(api)))
        .serve(addr)
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

    Ok(())
}
