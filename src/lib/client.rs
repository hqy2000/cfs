use std::error::Error;
use std::fmt::{Display, Formatter};

use tokio::runtime::Runtime;
use tonic::codegen::StdError;

use crate::proto::block::data_capsule_block::Block;
use crate::proto::block::DataCapsuleBlock;
use crate::proto::data_capsule::{GetRequest, LeafsRequest};
use crate::proto::data_capsule::data_capsule_client::DataCapsuleClient;

#[derive(Debug, Clone)]
struct ClientError {}

impl Display for ClientError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for ClientError {}

macro_rules! gen_client_methods {
    ($client:ident) => {
        pub fn connect<D>(addr: D) -> $client where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>, {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            let client = runtime.block_on(async {return  DataCapsuleClient::connect(addr).await.unwrap();});

            return $client {
                client,
                runtime
            };
        }

        pub fn get(&mut self, hash: &str) -> Result<DataCapsuleBlock, Box<dyn Error>> {
            return self.runtime.block_on(async {
                let request = tonic::Request::new(GetRequest {
                    block_hash: hash.to_string()
                });
                let response = self.client.get(request).await?;

                Ok(response.get_ref().clone().block.unwrap())
            });
        }
    };
}

pub struct BlockClient {
    client: DataCapsuleClient<tonic::transport::Channel>,
    runtime: Runtime,
}

pub struct INodeClient {
    client: DataCapsuleClient<tonic::transport::Channel>,
    runtime: Runtime,
}

impl BlockClient {
    gen_client_methods!(BlockClient);
    pub fn get_block(&mut self, hash: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = self.get(hash);
        if let Block::Data(data) = response.unwrap().block.unwrap() {
            Ok(data.data)
        } else {
            Err(Box::new(ClientError {}))
        }
    }
}

impl INodeClient {
    gen_client_methods!(INodeClient);


    pub fn get_leafs(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        return self.runtime.block_on(async {
            let request = tonic::Request::new(LeafsRequest {});
            let response = self.client.leafs(request).await?;
            Ok(response.get_ref().clone().leaf_ids)
        })
    }
}


// pub fn getSize(hash: &str) -> usize {
//     let response = get(hash);
//     let size = response.unwrap().block.unwrap().data.len();
//     // println!("{}", size);
//     return size.clone();
// }
