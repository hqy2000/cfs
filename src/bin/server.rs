use std::collections::HashMap;
use std::sync::Arc;

use futures::future::join_all;
use prost::Message;
use rsa::{
    pkcs1v15,
    pkcs8::{DecodePrivateKey, DecodePublicKey},
};
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer};
use tokio::sync::Mutex;
use tonic::transport::Server;

use lib::crypto::SignableBlock;
use lib::proto::block::{DataBlock, DataCapsuleBlock, DataCapsuleFileSystemBlock, Id, INodeBlock};
use lib::proto::block::data_capsule_file_system_block::Block;
use lib::proto::block::i_node_block::Kind;
use lib::proto::data_capsule::data_capsule_server::DataCapsuleServer;
use lib::proto::data_capsule::DataCapsuleServerData;
use lib::server::MyDataCapsule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_key = Arc::new(Mutex::new(pkcs1v15::VerifyingKey::<Sha256>::from_public_key_pem(include_str!("../../key/server_public.pem")).unwrap())); // todo: thread safe?

    let client1_public_pem = include_str!("../../key/client1_public.pem");
    let client1_signing_key = pkcs1v15::SigningKey::<Sha256>::from_pkcs8_pem(include_str!("../../key/client1_private.pem")).unwrap();

    let mut hqyId = Id {
        pub_key: Vec::from(client1_public_pem),
        uid: 1000, // hqy2000: 1000; hyc: 1002; yms: 1003
        signature: vec![],
    };
    hqyId.sign(&client1_signing_key);

    let data_capsule_addr = "127.0.0.1:50056".parse()?;
    let data_capsule = MyDataCapsule {
        data: Arc::new(Mutex::new(
            DataCapsuleServerData {
                content: HashMap::from(
                    [
                        ("file_hash1".to_string(),
                         DataCapsuleBlock {
                             prev_hash: "".to_string(),
                             fs: Some(DataCapsuleFileSystemBlock{
                                 prev_hash: "".to_string(),
                                 block: Some(Block::Data(DataBlock {
                                     // data: vec![u8::try_from('a').unwrap(); 512]
                                     data: Vec::from(include_str!("../../data/data1k.txt"))
                                 })),
                                 updated_by: None,
                                 signature: vec![],
                             }),
                             timestamp: 0,
                             signature: vec![],
                         }),  ("file_hash2".to_string(),
                               DataCapsuleBlock {
                                   prev_hash: "".to_string(),
                                   fs: Some(DataCapsuleFileSystemBlock{
                                       prev_hash: "".to_string(),
                                       block: Some(Block::Data(DataBlock {
                                           // data: vec![u8::try_from('b').unwrap(); 720 - 512]
                                           data: Vec::from(include_str!("../../data/data4g.txt"))
                                       })),
                                       updated_by: None,
                                       signature: vec![],
                                   }),
                                   timestamp: 0,
                                   signature: vec![],
                               })]
                ),
                leafs: Vec::new(),
            },
        )),
        verifying_key: server_key.clone()
    };

    let inode_capsule_addr = "127.0.0.1:50057".parse()?;
    let inode_capsule = MyDataCapsule {
        data: Arc::new(Mutex::new(
            DataCapsuleServerData {
                content: HashMap::from(
                    [
                        ("root".into(), DataCapsuleBlock {
                             prev_hash: "".to_string(),
                             fs: Some(DataCapsuleFileSystemBlock{
                                 prev_hash: "".to_string(),
                                 updated_by: None,
                                 signature: vec![],
                                 block: Some(Block::Inode(INodeBlock {
                                     filename: vec![],
                                     size: 0,
                                     kind: Kind::Directory.into(),
                                     hashes: vec![],
                                     write_allow_list: vec![hqyId.clone()],
                                 })),
                             }),
                            timestamp: 0,
                            signature: vec![],
                         }),
                        ("file".into(), DataCapsuleBlock {
                            prev_hash: "root".to_string(),
                            fs: Some(DataCapsuleFileSystemBlock{
                                prev_hash: "root".into(),
                                updated_by: None,
                                signature: vec![],
                                block: Some(Block::Inode(INodeBlock {
                                    filename: Vec::from("dc_file.txt"),
                                    size: 4294968320,
                                    kind: Kind::RegularFile.into(),
                                    hashes: vec!["file_hash1".into(), "file_hash2".into()],
                                    write_allow_list: vec![hqyId.clone()],
                                })),
                            }),
                            timestamp: 0,
                            signature: vec![],
                        }),
                        ("folder1".into(), DataCapsuleBlock {
                            prev_hash: "root".into(),
                            fs: Some(DataCapsuleFileSystemBlock{
                                prev_hash: "root".into(),
                                updated_by: None,
                                signature: vec![],
                                block: Some(Block::Inode(INodeBlock {
                                    filename: Vec::from("folder1"),
                                    size: 0,
                                    kind: Kind::Directory.into(),
                                    hashes: vec![],
                                    write_allow_list: vec![hqyId.clone()],
                                })),
                            }),
                            signature: vec![],
                            timestamp: 0,
                        })
                    ]
                ),
                leafs: vec!["file".into(), "folder1".into()],
            }
        )),
        verifying_key: server_key
    };

    let mut v = Vec::new();

    v.push(Server::builder()
        .add_service(DataCapsuleServer::new(data_capsule))
        .serve(data_capsule_addr));

    v.push(Server::builder()
        .add_service(DataCapsuleServer::new(inode_capsule))
        .serve(inode_capsule_addr));

    join_all(v).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    // use crate::server::server::data_capsule_client::DataCapsuleClient;
    // use crate::server::server::GetRequest;

    #[tokio::test]
    async fn test_get() -> Result<(), Box<dyn std::error::Error>> {
        // let mut client = DataCapsuleClient::connect("http://[::1]:50051").await?;
        //
        // let request = tonic::Request::new(GetRequest {
        //     block_hash: "testhash".to_string()
        // });
        //
        // let response = client.get(request).await?;
        //
        // println!("RESPONSE={:?}", response);
        Ok(())
    }
}