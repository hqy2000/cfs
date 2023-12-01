use std::collections::HashMap;
use std::time::UNIX_EPOCH;

use fuser::{FileAttr, FileType};
use fuser::FileType::Directory;

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
    pub block: INodeBlock,
    pub timestamp: i64,
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
            perm: self.get_perm(),
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
        } else if self.block.kind == Kind::RegularFile.into() {
            FileType::RegularFile
        } else {
            FileType::RegularFile // deleted file!!
        }
    }

    pub fn get_perm(&self) -> u16 {
        return if self.get_file_type() == Directory {
            0o755
        } else {
            0o644
        }
    }

    pub fn is_deleted(&self) -> bool {
        return self.block.kind == Kind::DeletedFolder.into() || self.block.kind == Kind::DeletedRegularFile.into();
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
                timestamp: block.timestamp
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


                let mut inode = INode{
                    hash: hash.clone(),
                    ino: self.inodes.len() as u64,
                    parent_ino: *parent_ino,
                    block: data,
                    timestamp: block.timestamp
                };


                let index = self.inodes.get(*parent_ino as usize).unwrap().1.iter().position(|x| x.block.filename == inode.block.filename);
                if let Some(idx) = index {
                    {
                        let prev_node = self.inodes.get(*parent_ino as usize).unwrap().1.get(idx).unwrap();
                        if prev_node.timestamp > inode.timestamp {
                            return; // we're having an older node, discard
                        }
                        inode.ino = prev_node.ino;
                    }

                    self.inodes[inode.ino as usize] = (inode.clone(), Vec::new()); // update local inode to the latest version
                    self.inodes.get_mut(*parent_ino as usize).unwrap().1.remove(idx);  // delete outdated inode from parent
                } else {
                    self.inodes.push((inode.clone(), Vec::new()));
                }
                self.inodes.get_mut(*parent_ino as usize).unwrap().1.push(inode.clone());
                self.hash_to_ino.insert(hash, inode.ino);
            } else {
                panic!();
            }
        }
    }
}