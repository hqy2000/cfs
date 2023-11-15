use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::ptr::null;
use std::time::UNIX_EPOCH;
use fuser::{FileAttr, FileType};
use ring::test::run;
use tokio::runtime::Runtime;
use tonic::codegen::StdError;
use crate::proto::block::{data_capsule_block, DataCapsuleBlock};
use crate::proto::block::data_capsule_block::Block;
use crate::proto::block::i_node_block::Kind;
use crate::proto::data_capsule::data_capsule_client::DataCapsuleClient;
use crate::proto::data_capsule::{GetRequest, GetResponse};

#[derive(Debug, Clone)]
struct ClientError {

}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for ClientError{}

macro_rules! gen_client_methods {
    ($client:ident) => {
        pub fn connect<D>(addr: D) -> $client where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>, {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            let client = runtime.block_on(async {return  DataCapsuleClient::connect(addr).await.unwrap()});

            return $client {
                client,
                runtime
            }
        }

        fn get(&mut self, hash: &str) -> Result<DataCapsuleBlock, Box<dyn Error>> {
            return self.runtime.block_on(async {
                let request = tonic::Request::new(GetRequest {
                    block_hash: hash.to_string()
                });
                let response = self.client.get(request).await?;

                Ok(response.get_ref().clone().block.unwrap())
            })
        }
    };
}

pub struct BlockClient {
    client: DataCapsuleClient<tonic::transport::Channel>,
    runtime: Runtime
}

pub struct INodeClient {
    client: DataCapsuleClient<tonic::transport::Channel>,
    runtime: Runtime
}

impl BlockClient {
    gen_client_methods!(BlockClient);
    pub fn get_block(&mut self, hash: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = self.get(hash);
        if let Block::Data(data) = response.unwrap().block.unwrap() {
            Ok(data.data)
        } else {
            Err(Box::new(ClientError{}))
        }
    }
}

impl INodeClient {
    gen_client_methods!(INodeClient);
    fn kind_to_type(&self, kind: i32) -> FileType {
        return if (kind == Kind::Directory.into()) {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }

    pub fn hash_to_ino(&self, hash: &str) -> u64 {
        if hash == "root" {
            return 1;
        } else {
            let mut s = DefaultHasher::new();
            hash.hash(&mut s);
            return s.finish();
        }

    }

    pub fn get_inode(&mut self, hash: &str) -> Result<FileAttr, Box<dyn Error>> {
        let block = self.get(hash).unwrap().block.unwrap();
        if let Block::Inode(data) = block {
            Ok(FileAttr{
                ino: self.hash_to_ino(hash),
                size: data.size,
                blocks: 0,
                atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: self.kind_to_type(data.kind),
                perm: 0o755,
                nlink: 2,
                uid: 501,
                gid: 20,
                rdev: 0,
                flags: 0,
                blksize: 512,
            })
        } else {
            Err(Box::new(ClientError{}))
        }
    }
}


// pub fn getSize(hash: &str) -> usize {
//     let response = get(hash);
//     let size = response.unwrap().block.unwrap().data.len();
//     // println!("{}", size);
//     return size.clone();
// }
