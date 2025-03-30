use common_proto::proto::hello_api_server::{HelloApi, HelloApiServer};
use common_proto::proto::{
    GetMessagesReply, GetMessagesRequest, HelloReply, HelloRequest, PublishReply, PublishRequest,
    GetPublishesReply, GetPublishesRequest, PublishedData,
};
use common_proto::envelope::Envelope;
use prost::Message as ProstMessage;
use log::{error, info};
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
    publish_topic: String,
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

pub struct KafkaService {
    kafka_producer: FutureProducer,
    topic: String,
    publish_topic: String,
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
            publish_topic: config.publish_topic.clone(),
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

    async fn publish_data(&self, request: &PublishRequest) -> Result<(), Status> {
        // Serialize the PublishRequest to bytes
        let request_bytes = request.encode_to_vec();
        
        // Create an envelope to wrap the serialized request
        let envelope = Envelope {
            payload: request_bytes,
            message_type: "hello.PublishRequest".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
            headers: std::collections::HashMap::new(),
        };
        
        // Serialize the envelope to bytes
        let envelope_bytes = envelope.encode_to_vec();
        
        // Create a key for the message (using a random UUID for uniqueness)
        let key = uuid::Uuid::new_v4().to_string();
        
        // Create and send the Kafka record
        let record = FutureRecord::to(&self.publish_topic)
            .key(&key)
            .payload(&envelope_bytes);

        self.kafka_producer
            .send(record, std::time::Duration::from_secs(5))
            .await
            .map_err(|err| Status::internal(format!("Failed to send publish message: {:?}", err)))?;

        info!("Data published with key {}", key);
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

#[derive(sqlx::FromRow)]
struct DbPublishedData {
    id: i32,
    topic: String,
    part: i32,
    kafkaoffset: i64,
    data: String,
    metadata: Option<String>,
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
    
    async fn get_publishes_from_db(
        &self,
        limit: i32,
        filter: Option<&str>,
    ) -> Result<Vec<proto::PublishedData>, sqlx::Error> {
        let query = match filter {
            Some(filter_text) => {
                sqlx::query_as::<_, DbPublishedData>(
                    "SELECT id, topic, part, kafkaoffset, data, metadata, created_at::text as created_at 
                     FROM published_data 
                     WHERE data LIKE $1 OR metadata LIKE $1
                     ORDER BY created_at DESC 
                     LIMIT $2",
                )
                .bind(format!("%{}%", filter_text))
                .bind(limit as i64)
            },
            None => {
                sqlx::query_as::<_, DbPublishedData>(
                    "SELECT id, topic, part, kafkaoffset, data, metadata, created_at::text as created_at 
                     FROM published_data 
                     ORDER BY created_at DESC 
                     LIMIT $1",
                )
                .bind(limit as i64)
            }
        };
        
        let published_data = query.fetch_all(&self.db_pool).await?;

        Ok(published_data
            .into_iter()
            .map(|p| proto::PublishedData {
                id: p.id,
                data: p.data,
                metadata: p.metadata.unwrap_or_default(),
                created_at: p.created_at,
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
    
    async fn publish(
        &self,
        request: Request<PublishRequest>,
    ) -> Result<Response<PublishReply>, Status> {
        info!("Received a publish request");
        let publish_request = request.into_inner();
        
        // Validate the request
        if publish_request.data.is_empty() {
            return Ok(Response::new(PublishReply {
                success: false,
                message: "Data field cannot be empty".to_string(),
            }));
        }
        
        // Publish the data to Kafka
        match self.kafka.publish_data(&publish_request).await {
            Ok(_) => {
                info!("Successfully published data");
                Ok(Response::new(PublishReply {
                    success: true,
                    message: "Data published successfully".to_string(),
                }))
            },
            Err(e) => {
                error!("Failed to publish data: {}", e);
                Ok(Response::new(PublishReply {
                    success: false,
                    message: format!("Failed to publish data: {}", e),
                }))
            }
        }
    }
    
    async fn get_publishes(
        &self,
        request: Request<GetPublishesRequest>,
    ) -> Result<Response<GetPublishesReply>, Status> {
        let req = request.into_inner();
        let filter = if req.filter.is_empty() { None } else { Some(req.filter.as_str()) };
        let limit = if req.limit <= 0 { 100 } else { req.limit }; // Default to 100 if not specified
        
        let publishes = self
            .get_publishes_from_db(limit, filter)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(GetPublishesReply { publishes }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = ServerConfig::load("api/config.json")?;
    let addr = "127.0.0.1:50051".parse()?;
    let api = MyHelloApi::new(&config)
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
