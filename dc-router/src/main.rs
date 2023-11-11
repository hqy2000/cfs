use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use crate::server::server::data_capsule_server::DataCapsuleServer;
use crate::server::server::{DataCapsuleBlock, DataCapsuleServerData, MyDataCapsule};

mod router;
mod server;
mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let router = MyDataCapsule {
        data: Arc::new(Mutex::new(
            DataCapsuleServerData {
                id: Vec::new(),
                write_pub_key: Vec::new(),
                content: HashMap::from(
                    [("testhash".to_string(), DataCapsuleBlock{
                        prev_hash: "".to_string(),
                        data: Vec::from("hello world on server!"),
                        signature: vec![],
                    })]
                ),
                leafs: Vec::new()
            }
        )),
    };

    Server::builder()
        .add_service(DataCapsuleServer::new(router))
        .serve(addr)
        .await?;

    Ok(())
}
