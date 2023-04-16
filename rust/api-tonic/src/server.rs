use tonic::{transport::Server, Request, Response, Status};
use log::{info, error};
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
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Received a request: {:?}", request);

        let reply = proto::HelloReply {
            // We must use .into_inner() as the fields of gRPC requests and responses are private
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "127.0.0.1:50051".parse().unwrap();
    let api = MyHelloApi::default();

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