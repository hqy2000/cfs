use std::sync::Arc;

use rsa::pkcs1v15::VerifyingKey;
use rsa::sha2::Sha256;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use crate::crypto::SignableBlock;

use crate::proto::data_capsule::{DataCapsuleServerData, GetRequest, GetResponse, LeafsRequest, LeafsResponse, PutRequest, PutResponse};
use crate::proto::data_capsule::data_capsule_server::DataCapsule;

#[derive(Debug)]
pub struct MyDataCapsule {
    pub data: Arc<Mutex<DataCapsuleServerData>>,
    pub verifying_key: VerifyingKey<Sha256>,
    pub enable_crypto: bool,
}

#[tonic::async_trait]
impl DataCapsule for MyDataCapsule {
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        println!("Got a get request: {:?}", request.get_ref().block_hash);
        let reply = GetResponse {
            block: self.data.lock().await.content.get(&request.into_inner().block_hash).cloned()
        };
        Ok(Response::new(reply))
    }

    async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        println!("Got a put request.");

        let request = request.into_inner();
        let mut block = request.block.unwrap();
        if self.enable_crypto {
            block.validate(&self.verifying_key);
        }
        let hash = block.hash();

        let mut mutex = self.data.lock().await;

        if mutex.content.contains_key(&hash) {
            Ok(Response::new(PutResponse {
                success: false,
                hash: "".into()
            }))
        } else {
            let prev_hash = block.prev_hash.clone();
            mutex.content.insert(hash.clone(), block);
            mutex.leafs.push(hash.clone());

            let index = mutex.leafs.iter().position(|x| *x == prev_hash);
            if index.is_some() {
                mutex.leafs.remove(index.unwrap());
            }

            Ok(Response::new(PutResponse {
                success: true,
                hash
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