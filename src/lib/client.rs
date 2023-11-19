use std::error::Error;
use std::fmt::{Display, Formatter};
use duplicate::duplicate_item;

use tokio::runtime::Runtime;
use tonic::codegen::StdError;

use crate::proto::block::{DataCapsuleBlock, DataCapsuleFileSystemBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::data_capsule::{GetRequest, LeafsRequest};
use crate::proto::data_capsule::data_capsule_client::DataCapsuleClient;
use crate::proto::middleware::{PutDataRequest, PutINodeRequest};
use crate::proto::middleware::middleware_client::MiddlewareClient;

#[derive(Debug, Clone)]
struct ClientError {}

impl Display for ClientError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for ClientError {}

#[duplicate_item(
    T C;
    [BlockClient] [DataCapsuleClient];
    [INodeClient] [DataCapsuleClient];
    [FSMiddlewareClient] [MiddlewareClient];
)]
pub struct T {
    client: C<tonic::transport::Channel>,
    runtime: Runtime,
}

#[duplicate_item(
    T C;
    [BlockClient] [DataCapsuleClient];
    [INodeClient] [DataCapsuleClient];
    [FSMiddlewareClient] [MiddlewareClient];
)]
impl T {
    pub fn connect<D>(addr: D) -> T where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>, {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let client = runtime.block_on(async { return C::connect(addr).await.unwrap(); });

        return T {
            client,
            runtime,
        };
    }
}

#[duplicate_item(T; [BlockClient]; [INodeClient])]
impl T {
    pub fn get(&mut self, hash: &str) -> Result<DataCapsuleBlock, Box<dyn Error>> {
        return self.runtime.block_on(async {
            let request = tonic::Request::new(GetRequest {
                block_hash: hash.to_string()
            });
            let response = self.client.get(request).await?;

            Ok(response.get_ref().clone().block.unwrap())
        });
    }

    pub fn get_leafs(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        return self.runtime.block_on(async {
            let request = tonic::Request::new(LeafsRequest {});
            let response = self.client.leafs(request).await?;
            Ok(response.get_ref().clone().leaf_ids)
        });
    }
}

impl BlockClient {
    pub fn get_block(&mut self, hash: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = self.get(hash);
        if let Block::Data(data) = response.unwrap().fs.unwrap().block.unwrap() {
            Ok(data.data)
        } else {
            Err(Box::new(ClientError {}))
        }
    }
}

impl FSMiddlewareClient {
    pub fn put_inode(&mut self, block: DataCapsuleFileSystemBlock) -> Result<String, Box<dyn Error>> {
        return self.runtime.block_on(async {
            if let Block::Inode(ref _data) = block.block.as_ref().unwrap() {
                let request = tonic::Request::new(PutINodeRequest {
                    block: Some(block)
                });
                let response = self.client.put_i_node(request).await?;
                Ok(response.get_ref().clone().hash.unwrap())
            } else {
                panic!("received data in put_inode")
            }
        });
    }

    pub fn put_data(&mut self, block: DataCapsuleFileSystemBlock, ref_inode_hash: String) -> Result<String, Box<dyn Error>> {
        return self.runtime.block_on(async {
            if let Block::Data(ref _data) = block.block.as_ref().unwrap() {
                let request = tonic::Request::new(PutDataRequest {
                    block: Some(block),
                    inode_hash: ref_inode_hash,
                });
                let response = self.client.put_data(request).await?;
                Ok(response.get_ref().clone().hash.unwrap())
            } else {
                panic!("received inode in put_data")
            }
        });
    }
}