use tonic::{transport::Server, Request, Response, Status};

use proto::hello_api_server::{HelloApi, HelloApiServer};
use proto::{HelloReply, HelloRequest};

mod proto {
    tonic::include_proto!("hello");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("hello_descriptor");
}

#[derive(Debug, Default)]
pub struct MyHelloApi {}

#[tonic::async_trait]
impl HelloApi for MyHelloApi {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> { // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);

        let reply = proto::HelloReply {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse().unwrap();
    let api = MyHelloApi::default();

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    println!("Server is online at {}", addr);

    Server::builder()
        .add_service(reflection)
        .add_service(HelloApiServer::new(api))
        .serve(addr)
        .await?;

    Ok(())
}