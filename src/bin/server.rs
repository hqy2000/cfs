use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use lib::proto::block::data_capsule_block::Block;
use lib::proto::block::{DataBlock, DataCapsuleBlock};
use lib::proto::data_capsule::data_capsule_server::DataCapsuleServer;
use lib::proto::data_capsule::DataCapsuleServerData;
use lib::server::MyDataCapsule;

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
                        block: Some(Block::Data(DataBlock{
                            data: Vec::from("dc server resp")
                        })),
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

#[cfg(test)]
mod tests {
    use crate::server::server::data_capsule_client::DataCapsuleClient;
    use crate::server::server::GetRequest;

    #[tokio::test]
    async fn test_get() -> Result<(), Box<dyn std::error::Error>>  {
        let mut client = DataCapsuleClient::connect("http://[::1]:50051").await?;

        let request = tonic::Request::new(GetRequest {
            block_hash: "testhash".to_string()
        });

        let response = client.get(request).await?;

        println!("RESPONSE={:?}", response);
        Ok(())
    }
}