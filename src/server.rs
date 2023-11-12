pub mod server {
    tonic::include_proto!("data_capsule");
    use std::sync::Arc;
    use data_capsule_server::DataCapsule;
    use tonic::{Request, Response, Status};
    use ring::digest::{Context, SHA256};
    use data_encoding::{HEXLOWER};
    use tokio::sync::Mutex;

    #[derive(Debug, Default)]
    pub struct MyDataCapsule {
        pub(crate) data: Arc<Mutex<DataCapsuleServerData>>
    }

    #[tonic::async_trait]
    impl DataCapsule for MyDataCapsule {
        async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
            println!("Got a get request: {:?}", request);
            let reply = GetResponse {
                block: self.data.lock().await.content.get(&request.into_inner().block_hash).cloned()
            };
            Ok(Response::new(reply))
        }

        async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
            println!("Got a put request: {:?}", request);

            let request = request.into_inner();
            let block = request.block.unwrap();
            let ref_hash = request.hash;

            let mut context = Context::new(&SHA256);
            context.update(&block.prev_hash.as_bytes());
            context.update(&block.data);
            let hash = HEXLOWER.encode(context.finish().as_ref());

            let mut mutex = self.data.lock().await;

            if hash != ref_hash || mutex.content.contains_key(&hash) {
                Ok(Response::new(PutResponse {
                    success: false
                }))
            } else {
                mutex.content.insert(hash.clone(), block);
                mutex.leafs.push(hash.clone());

                let index = mutex.leafs.iter().position(|x| *x == hash);
                if index.is_some() {
                    mutex.leafs.remove(index.unwrap());
                }

                Ok(Response::new(PutResponse {
                    success: true
                }))
            }
        }

        async fn leafs(&self, _request: Request<LeafsRequest>) -> Result<Response<LeafsResponse>, Status> {
            let reply = LeafsResponse {
                leaf_ids: self.data.lock().await.leafs.clone()
            };
            Ok(Response::new(reply))
        }
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use crate::server::server::data_capsule_server::DataCapsuleServer;
use crate::server::server::{DataCapsuleBlock, DataCapsuleServerData, MyDataCapsule};

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