use log::{error, info};
use proto::hello_api_server::{HelloApi, HelloApiServer};
use proto::{HelloReply, HelloRequest};
use rdkafka::util::Timeout;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use std::net::SocketAddr;
use tonic::{transport::Server, Request, Response, Status};

const SEND_TIMEOUT_MS: u64 = 5000;

mod proto {
    tonic::include_proto!("hello");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("hello_descriptor");
}

pub struct KafkaService {
    kafka_producer: FutureProducer,
    default_topic: &'static str,
}

impl Default for KafkaService {
    fn default() -> Self {
        Self::new(9092)
    }
}

impl KafkaService {
    pub fn new(port: i32) -> KafkaService {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", format!("localhost:{}", port))
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        KafkaService {
            kafka_producer: producer,
            default_topic: "default-topic",
        }
    }

    async fn publish(&self, name: String) -> Result<(), Status> {
        let payload = format!("Hello {}", name);
        let record = FutureRecord::to(self.default_topic)
            .key(&name)
            .payload(&payload)
            .timestamp(chrono::Utc::now().timestamp_millis());

        match self.kafka_producer
            .send(record, Timeout::After(std::time::Duration::from_millis(SEND_TIMEOUT_MS)))
            .await
        {
            Ok(_) => {
                info!("Message published for {}", name);
                Ok(())
            }
            Err((err, _)) => {
                error!("Failed to publish message for {}: {:?}", name, err);
                Err(Status::internal(format!("Failed to send message: {:?}", err)))
            }
        }
    }
}

pub struct MyHelloApi {
    kafka: KafkaService,
}

impl MyHelloApi {
    fn new(kafka_service: KafkaService) -> MyHelloApi {
        MyHelloApi {
            kafka: kafka_service,
        }
    }
}

#[tonic::async_trait]
impl HelloApi for MyHelloApi {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Received a request: {:?}", request);
        let name = request.into_inner().name;

        self.kafka
            .publish(name.clone())
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

    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let kafka: KafkaService = KafkaService::new(9092);
    let api: MyHelloApi = MyHelloApi::new(kafka);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_say_hello() {
        let kafka = KafkaService::new(9092);
        let api = MyHelloApi::new(kafka);
        
        let request = Request::new(HelloRequest {
            name: "Test User".to_string(),
        });

        let response = api.say_hello(request).await.unwrap();
        assert_eq!(
            response.into_inner().message,
            "Hello Test User!"
        );
    }

    #[tokio::test]
    async fn test_kafka_publish() {
        let kafka = KafkaService::new(9092);
        let result = kafka.publish("Test User".to_string()).await;
        assert!(result.is_ok());
    }
}
