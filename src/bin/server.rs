use std::collections::HashMap;
use std::sync::Arc;
use futures::future::join_all;
use tokio::sync::Mutex;
use tonic::transport::Server;
use lib::proto::block::data_capsule_file_system_block::Block;
use lib::proto::block::{DataBlock, DataCapsuleBlock, DataCapsuleFileSystemBlock, INodeBlock};
use lib::proto::block::i_node_block::Kind;
use lib::proto::data_capsule::data_capsule_server::DataCapsuleServer;
use lib::proto::data_capsule::DataCapsuleServerData;
use lib::server::MyDataCapsule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_capsule_addr = "[::1]:50051".parse()?;
    let data_capsule = MyDataCapsule {
        data: Arc::new(Mutex::new(
            DataCapsuleServerData {
                content: HashMap::from(
                    [
                        ("file_hash".to_string(),
                         DataCapsuleBlock {
                             prev_hash: "".to_string(),
                             fs: Some(DataCapsuleFileSystemBlock{
                                 prev_hash: "".to_string(),
                                 block: Some(Block::Data(DataBlock {
                                     data: Vec::from("dc server resp lololololol")
                                 })),
                                 updated_by: None,
                                 signature: vec![],
                             }),
                             timestamp: 0,
                             signature: vec![],
                         })]
                ),
                leafs: Vec::new(),
            }
        )),
    };

    let inodeCapsuleAddr = "[::1]:50052".parse()?;
    let inodeCapsule = MyDataCapsule {
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
                                     write_allow_list: vec![],
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
                                    size: 26,
                                    kind: Kind::RegularFile.into(),
                                    hashes: vec!["file_hash".into()],
                                    write_allow_list: vec![],
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
                                    write_allow_list: vec![],
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
    };

    let mut v = Vec::new();

    v.push(Server::builder()
        .add_service(DataCapsuleServer::new(data_capsule))
        .serve(data_capsule_addr));

    v.push(Server::builder()
        .add_service(DataCapsuleServer::new(inodeCapsule))
        .serve(inodeCapsuleAddr));

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