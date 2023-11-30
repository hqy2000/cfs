use std::collections::HashMap;
use std::time::UNIX_EPOCH;

use fuser::{FileAttr, FileType};

use crate::client::INodeClient;
use crate::proto::block::INodeBlock;
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

pub struct INodeCache {
    pub client: INodeClient,
    pub inodes: Vec<(INode, Vec<INode>)>, // Vec<Node, Children>
    pub hash_to_ino: HashMap<String, u64> // Hash -> INode.ino
}

#[derive(Clone)]
pub struct INode {
    pub hash: String,
    pub ino: u64,
    pub parent_ino: u64,
    pub block: INodeBlock
}

impl INode {
    pub fn to_file_attr(&self) -> FileAttr {
        FileAttr{
            ino: self.ino,
            size: self.block.size,
            blocks: 0,
            atime: UNIX_EPOCH, // 1970-01-01 00:00:00
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: self.get_file_type(),
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    pub fn get_file_type(&self) -> FileType {
        return if self.block.kind == Kind::Directory.into() {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }
}

impl INodeCache {
    pub fn new(client: INodeClient, root: String) -> INodeCache {
        let mut cache = INodeCache {
            client,
            inodes: Vec::new(),
            hash_to_ino: HashMap::new()
        };
        cache.build(root);
        return cache;
    }

    fn build(&mut self, root: String) {
        let block = self.client.get(&root).unwrap();
        if let Block::Inode(data) = block.fs.unwrap().block.unwrap() {
            let inode = INode{
                hash: root.clone(),
                ino: 1,
                parent_ino: 1,
                block: data,
            };

            self.inodes.push((inode.clone(), Vec::new()));
            self.inodes.push((inode, Vec::new()));
            self.hash_to_ino.insert(root, 1);

            let leafs = self.client.get_leafs().unwrap();
            for leaf in leafs {
                self.resolve(leaf);
            }
        } else {
            panic!();
        }
    }

    pub fn num_inodes(&self) -> u64 {
        return self.inodes.len() as u64;
    }

    pub fn get_inode(&self, ino: u64) -> INode {
        let inode = self.inodes.get(ino as usize);
        return inode.unwrap().clone().0;
    }

    pub fn get_ino(&self, hash: String) -> u64 {
        return *self.hash_to_ino.get(&hash).unwrap();
    }

    pub fn get_sub_inodes(&self, ino: u64) -> Vec<INode> {
        let inode = self.inodes.get(ino as usize);
        return inode.unwrap().clone().1;
    }

    pub fn resolve(&mut self, hash: String)  {
        if !self.hash_to_ino.contains_key(&hash) {
            let block = self.client.get(&hash).unwrap();

            if !self.hash_to_ino.contains_key(&block.prev_hash) {
                self.resolve(block.prev_hash.clone());
            }

            if let Block::Inode(data) = block.fs.unwrap().block.unwrap() {
                let parent_ino = self.hash_to_ino.get(&block.prev_hash).unwrap();

                let inode = INode{
                    hash: hash.clone(),
                    ino: self.inodes.len() as u64,
                    parent_ino: *parent_ino,
                    block: data,
                };

                self.inodes.push((inode.clone(), Vec::new()));
                self.inodes.get_mut(*parent_ino as usize).unwrap().1.push(inode.clone());
                self.hash_to_ino.insert(hash, inode.ino);
            } else {
                panic!();
            }
        }
    }
}