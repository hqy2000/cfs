use std::collections::HashMap;
use std::fs;

use clap::{Arg, Command};
use prost::Message;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;

use lib::crypto::SignableBlock;
use lib::proto::block::{DataBlock, DataCapsuleBlock, DataCapsuleFileSystemBlock, Id, INodeBlock};
use lib::proto::block::data_capsule_file_system_block::Block;
use lib::proto::block::i_node_block::Kind;
use lib::proto::data_capsule::DataCapsuleServerData;

/* This program initiates empty server files.
 */
fn main() {
    let matches = Command::new("gen")
        .arg(
            Arg::new("CLIENT_SIGNING_KEY")
                .required(true)
                .index(1)
                .help("Path to the client's signing key to initialize the ACL")
        ).arg(
        Arg::new("CLIENT_VERIFYING_KEY")
            .required(true)
            .index(2)
            .help("Path to the client's verifying key to initialize the ACL")
        ).arg(
            Arg::new("UID")
                .required(true)
                .index(3)
                .help("UID to initialize the ACL")
        ).arg(
            Arg::new("SERVER_SIGNING_KEY")
                .required(true)
                .index(4)
                .help("Path to the server's signing key to initialize the first block")
        ).get_matches();

    let client_signing_key = pkcs1v15::SigningKey::<Sha256>::read_pkcs8_pem_file(matches.get_one::<String>("CLIENT_SIGNING_KEY").unwrap()).unwrap();
    let client_verifying_key = fs::read(matches.get_one::<String>("CLIENT_VERIFYING_KEY").unwrap()).unwrap();
    let server_signing_key = pkcs1v15::SigningKey::<Sha256>::read_pkcs8_pem_file(matches.get_one::<String>("SERVER_SIGNING_KEY").unwrap()).unwrap();
    let empty =  DataCapsuleServerData {
        content: HashMap::new(),
        leafs: Vec::new(),
    };

    let mut id = Id {
        pub_key: client_verifying_key,
        uid: matches.get_one::<String>("UID").unwrap().parse().unwrap(),
        signature: vec![],
    };
    id.sign(&client_signing_key);
    
    // --------------- initiate empty data. --------------------
    let mut data_server_file = empty.clone();

    let mut data_block = DataCapsuleFileSystemBlock {
        prev_hash: "".into(),
        updated_by: Some(id.clone()),
        signature: vec![],
        block: Some(Block::Data(DataBlock {
            data: vec![]
        })),
    };
    data_block.sign(&client_signing_key);
    
    let mut block = DataCapsuleBlock {
        prev_hash: "".into(),
        fs: Some(data_block),
        timestamp: 0,
        signature: vec![],
    };
    block.sign(&server_signing_key);
    
    let data_hash = block.hash();
    data_server_file.content.insert(data_hash.clone(), block);
    data_server_file.leafs.push(data_hash.clone());

    let mut buf = vec![];
    data_server_file.encode(&mut buf).unwrap();
    fs::write("data_server.bin", buf).expect("Unable to serialize data to local file");
    println!("Successfully written the empty DataBlock server file with root hash {}.", data_hash);
    
    // --------------- initiate and sign root inode. --------------------
    let mut inode_server_file = empty.clone();

    let mut inode_block = DataCapsuleFileSystemBlock {
        prev_hash: "".into(),
        updated_by: Some(id.clone()),
        signature: vec![],
        block: Some(Block::Inode(INodeBlock {
            filename: "".into(),
            size: 0,
            kind: Kind::Directory.into(),
            hashes: vec![],
            write_allow_list: vec![id.clone()],
        })),
    };
    inode_block.sign(&client_signing_key);

    
    let mut block = DataCapsuleBlock {
        prev_hash: "".into(),
        fs: Some(inode_block),
        timestamp: 0,
        signature: vec![],
    };
    block.sign(&server_signing_key);

    let data_hash = block.hash();
    inode_server_file.content.insert(data_hash.clone(), block);
    inode_server_file.leafs.push(data_hash.clone());

    let mut buf = vec![];
    inode_server_file.encode(&mut buf).unwrap();
    fs::write("inode_server.bin", buf).expect("Unable to serialize data to local file");
    println!("Successfully written the empty InodeBlock server file with root hash {}.", data_hash);
}
