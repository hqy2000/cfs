use tonic::transport::Server;
use crate::server::server::data_capsule_server::DataCapsuleServer;
use crate::server::server::MyDataCapsule;

mod router;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let router = MyDataCapsule::default();

    Server::builder()
        .add_service(DataCapsuleServer::new(router))
        .serve(addr)
        .await?;

    Ok(())
}
