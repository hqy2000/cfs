use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::sync::Mutex;

use duplicate::duplicate_item;
use lru::LruCache;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::Sha256;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tonic::transport::{Channel, ClientTlsConfig};

use crate::crypto::SignableBlock;
use crate::proto::block::{DataCapsuleBlock, DataCapsuleFileSystemBlock, Id};
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
)]
pub struct T {
    client: C<tonic::transport::Channel>,
    runtime: Runtime,
    cache: Mutex<LruCache<String, DataCapsuleBlock>>,
}

#[duplicate_item(
T C;
[BlockClient] [DataCapsuleClient];
[INodeClient] [DataCapsuleClient];
)]
impl T {
    pub fn connect(addr: &'static str, tls_config: ClientTlsConfig, cache_size: usize) -> T {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let channel = runtime.block_on(async {
            return Channel::from_static(addr)
                .tls_config(tls_config).unwrap()
                .connect()
                .await;
        }).unwrap();

        let client = C::new(channel);

        return T {
            client,
            runtime,
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap())),
        };
    }
}

#[duplicate_item(T; [BlockClient]; [INodeClient])]
impl T {
    pub fn get(&mut self, hash: &str) -> Result<DataCapsuleBlock, Box<dyn Error>> {
        let mut cache = self.cache.lock().unwrap();

        return if let Some(block) = cache.get(hash) {
            Ok(block.clone())
        } else {
            drop(cache);
            self.runtime.block_on(async {
                let request = tonic::Request::new(GetRequest {
                    block_hash: hash.to_string()
                });
                let response = self.client.get(request).await?;
                let block = response.get_ref().clone().block.unwrap();

                self.cache.lock().unwrap().put(hash.to_string(), block.clone());
                Ok(block)
            })
        }
    }

    pub async fn get_async(&self, hash: String) -> Result<DataCapsuleBlock, Box<dyn Error + Send + Sync>> {
        let mut cache = self.cache.lock().unwrap();

        return if let Some(block) = cache.get(&hash) {
            Ok(block.clone())
        } else {
            drop(cache);

            let mut client = self.client.clone();
            let request = tonic::Request::new(GetRequest {
                block_hash: hash.to_string()
            });

            let handle: JoinHandle<Result<DataCapsuleBlock, Box<dyn Error + Send + Sync>>> = self.runtime.spawn(async move {
                let response = client.get(request).await?;
                let block = response.get_ref().clone().block.unwrap();
                Ok(block)
            });

            let block = handle.await??;
            self.cache.lock().unwrap().put(hash.to_string(), block.clone());
            Ok(block)
        }
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
    pub async fn get_block(&self, hash: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = self.get_async(hash).await;
        if let Block::Data(data) = response.unwrap().fs.unwrap().block.unwrap() {
            Ok(data.data)
        } else {
            Err(Box::new(ClientError {}))
        }
    }
}

pub struct FSMiddlewareClient {
    client: MiddlewareClient<tonic::transport::Channel>,
    runtime: Runtime,
    public_key_pkcs8: String,
    signing_key: SigningKey<Sha256>,
}

impl FSMiddlewareClient {
    pub fn connect(addr: &'static str, tls_config: ClientTlsConfig, public_key_pkcs8: String, signing_key: SigningKey<Sha256>) -> FSMiddlewareClient {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let channel = runtime.block_on(async {
            return Channel::from_static(addr)
                .tls_config(tls_config).unwrap()
                .connect()
                .await;
        }).unwrap();

        let client = MiddlewareClient::new(channel);

        return FSMiddlewareClient {
            client,
            runtime,
            public_key_pkcs8,
            signing_key,
        };
    }

    pub async fn put_inode(&self, mut block: DataCapsuleFileSystemBlock) -> Result<String, Box<dyn Error + Send + Sync>> {
        block.sign(&self.signing_key.clone());
        if let Block::Inode(ref _data) = block.block.as_ref().unwrap() {
            let mut client = self.client.clone();
            let request = tonic::Request::new(PutINodeRequest {
                block: Some(block)
            });
            let handle: JoinHandle<Result<String, Box<dyn Error + Send + Sync>>> = self.runtime.spawn(async move {
                let response = client.put_i_node(request).await?;
                Ok(response.get_ref().clone().hash.unwrap())
            });

            Ok(handle.await??)
        } else {
            panic!("received data in put_inode")
        }
    }

    pub async fn put_data(&self, mut block: DataCapsuleFileSystemBlock, ref_inode_hash: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        block.sign(&self.signing_key.clone());
        if let Block::Data(ref _data) = block.block.as_ref().unwrap() {
            let mut client = self.client.clone();
            let request = tonic::Request::new(PutDataRequest {
                block: Some(block),
                inode_hash: ref_inode_hash,
            });
            let handle: JoinHandle<Result<String, Box<dyn Error + Send + Sync>>> = self.runtime.spawn(async move {
                let response = client.put_data(request).await?;
                Ok(response.get_ref().clone().hash.unwrap())
            });
            Ok(handle.await??)
        } else {
            panic!("received inode in put_data")
        }
    }


    pub fn get_id(&self, uid: u64) -> Id {
        let mut id = Id {
            pub_key: Vec::from(self.public_key_pkcs8.clone()),
            uid,
            signature: vec![],
        };
        id.sign(&self.signing_key);
        return id;
    }
}