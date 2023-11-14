use tonic::transport::Server;
use crate::middleware::middleware::middleware_server::MiddlewareServer;
use crate::middleware::middleware::MyMiddleware;

mod middleware;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50052".parse()?;
    let router = MyMiddleware::default();

    Server::builder()
        .add_service(MiddlewareServer::new(router))
        .serve(addr)
        .await?;

    Ok(())
}