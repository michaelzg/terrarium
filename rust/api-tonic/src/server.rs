use std::net::SocketAddr;
use std::time::Duration;

use log::{error, info};
use rdkafka::{config::ClientConfig, producer::{FutureProducer, FutureRecord}};
use rdkafka::util::Timeout;
use tonic::{Request, Response, Status, transport::Server};

use proto::{HelloReply, HelloRequest};
use proto::hello_api_server::{HelloApi, HelloApiServer};

mod proto {
    tonic::include_proto!("hello");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("hello_descriptor");
}


pub struct KafkaService {
    kafkaProducer: FutureProducer,
    defaultTopic: &'static str,
}

impl KafkaService {
    pub fn new() -> KafkaService {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Producer creation error");

        KafkaService {
            kafkaProducer: producer,
            defaultTopic: "default-topic",
        }
    }

    async fn publish(&self, name: &String) -> Result<(), Status> {
        let payload = format!("Hello {}", name);
        let record =
            FutureRecord::to(self.defaultTopic)
            .key(name)
            .payload(&payload);

        self.kafkaProducer.send(record, Timeout::Never)
            .await
            .map_err(|err| Status::internal(format!("Failed to send message: {:?}", err)))?;

        info!("Message published for {}", name);
        Ok(())
    }
}

pub struct MyHelloApi {
    kafka: KafkaService,
}

impl MyHelloApi {
    fn new(kafka_service: KafkaService) -> MyHelloApi {
        MyHelloApi { kafka: kafka_service }
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
            .publish(&name)
            .await
            .map_err(|err| Status::internal(format!("Failed to send message: {:?}", err)))?;

        let reply = proto::HelloReply {
            message: format!("Hello {}!", name).into(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let kafka: KafkaService = KafkaService::new();
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