use std::sync::Arc;

use data_encoding::HEXLOWER;
use prost::Message;
use ring::digest::{Context, SHA256};
use rsa::pkcs1v15::VerifyingKey;
use rsa::sha2::Sha256;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::proto::data_capsule::{DataCapsuleServerData, GetRequest, GetResponse, LeafsRequest, LeafsResponse, PutRequest, PutResponse};
use crate::proto::data_capsule::data_capsule_server::DataCapsule;

#[derive(Debug)]
pub struct MyDataCapsule {
    pub data: Arc<Mutex<DataCapsuleServerData>>,
    pub verifying_key: Arc<Mutex<VerifyingKey<Sha256>>>,
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

        let mut context = Context::new(&SHA256);
        let mut buf = vec![];
        block.encode(&mut buf).unwrap();
        context.update(&buf);
        let hash = HEXLOWER.encode(context.finish().as_ref());

        let mut mutex = self.data.lock().await;

        if mutex.content.contains_key(&hash) {
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