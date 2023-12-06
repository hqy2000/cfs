use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::sync::Arc;

use futures::executor::block_on;

use crate::client::{BlockClient, FSMiddlewareClient, INodeClient};
use crate::inode::INode;
use crate::proto::block::{DataCapsuleBlock, DataCapsuleFileSystemBlock, INodeBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

pub struct Cache {
    inode_client: INodeClient,
    block_client: Arc<BlockClient>,
    middleware_client: Option<Arc<FSMiddlewareClient>>,
    inodes: Vec<(INode, Vec<INode>)>, // Vec<Node, Children>
    hash_to_ino: HashMap<String, u64>, // Hash -> INode.ino
    data_root: String
}

impl Cache {
    pub fn new(
        client: INodeClient,
        block_client: BlockClient,
        middleware: Option<FSMiddlewareClient>,
        inode_root: String,
        data_root: String) -> Cache {
        let mut cache = Cache {
            inode_client: client,
            block_client: Arc::new(block_client),
            middleware_client: None,
            inodes: Vec::new(),
            hash_to_ino: HashMap::new(),
            data_root
        };
        if let Some(middleware) = middleware {
            cache.middleware_client = Some(Arc::new(middleware));
        }
        cache.build(inode_root);
        return cache;
    }

    fn build(&mut self, root: String) {
        let block = self.inode_client.get(&root).unwrap();
        if let Block::Inode(data) = block.fs.unwrap().block.unwrap() {
            let inode = INode{
                hash: root.clone(),
                ino: 1,
                parent_hash: root.clone(),
                block: data,
                timestamp: block.timestamp,
                block_client: self.block_client.clone(),
                middleware_client: self.middleware_client.clone(),
                journal: HashMap::new(),
                prev_data_hash: self.data_root.clone()
            };

            self.inodes.push((inode.clone(), Vec::new()));
            self.inodes.push((inode, Vec::new()));
            self.hash_to_ino.insert(root, 1);

            let leafs = self.inode_client.get_leafs().unwrap();
            for leaf in leafs {
                self.resolve(leaf);
            }
        } else {
            panic!();
        }
    }

    pub fn find_child_node(&self, ino: u64, name: &OsStr) -> Option<INode> {
        if ino > self.num_inodes() {
            return None;
        }

        let nodes = self.get_inode(ino).1;
        return nodes.iter().find(|x| OsStr::from_bytes(&x.block.filename) == name && !x.is_deleted()).cloned();
    }

    pub fn delete(&mut self, uid: u32, ino: u64) {
        let mut block = self.get_inode(ino).0.block;
        if block.kind == Kind::Directory.into() {
            block.kind = Kind::DeletedFolder.into();
        } else if block.kind == Kind::RegularFile.into() {
            block.kind = Kind::DeletedRegularFile.into();
        }
        block.size = 0;
        block.hashes = vec![];

        self.update(uid, ino, block);
    }

    pub fn update(&mut self, uid: u32, ino: u64, block: INodeBlock) {
        let block = DataCapsuleFileSystemBlock {
            prev_hash:self.get_inode(ino).0.parent_hash,
            block: Some(Block::Inode(block)),
            updated_by: Some(self.middleware_client.clone().unwrap().get_id(uid as u64)),
            signature: vec![],
        };
        let response = block_on(self.middleware_client.clone().unwrap().put_inode(block)).unwrap();
        self.resolve_block(response.hash.unwrap(), response.block.unwrap());
    }

    pub fn create(&mut self, uid: u32, parent_ino: u64, name: &OsStr, kind: Kind) {
        let parent_block = self.get_inode(parent_ino).0;

        let inode_block = INodeBlock {
            filename: Vec::from(name.to_str().unwrap()),
            size: 0,
            kind: kind.into(),
            hashes: vec![],
            write_allow_list: parent_block.block.write_allow_list.clone(),
        };

        let block = DataCapsuleFileSystemBlock {
            prev_hash: self.get_inode(parent_ino).0.hash,
            block: Some(Block::Inode(inode_block)),
            updated_by: Some(self.middleware_client.clone().unwrap().get_id(uid as u64)),
            signature: vec![],
        };
        // block.sign(self.signing_key.as_ref().unwrap());
        let response = block_on(self.middleware_client.clone().unwrap().put_inode(block.clone())).unwrap();
        self.resolve_block(response.hash.unwrap(), response.block.unwrap());
    }

    pub fn num_inodes(&self) -> u64 {
        return self.inodes.len() as u64;
    }

    pub fn get_inode(&self, ino: u64) -> (INode, Vec<INode>) {
        let inode = self.inodes.get(ino as usize);
        return inode.unwrap().clone();
    }

    pub fn get_ino(&self, hash: String) -> u64 {
        return *self.hash_to_ino.get(&hash).unwrap();
    }

    fn resolve(&mut self, hash: String) {
        if !self.hash_to_ino.contains_key(&hash) {
            let block = self.inode_client.get(&hash).unwrap();
            self.resolve_block(hash, block)
        }
    }

    fn resolve_block(&mut self, hash: String, block: DataCapsuleBlock) {
        if !self.hash_to_ino.contains_key(&block.prev_hash) {
            self.resolve(block.prev_hash.clone());
        }

        if let Block::Inode(data) = block.fs.unwrap().block.unwrap() {
            let parent_ino = self.hash_to_ino.get(&block.prev_hash).unwrap();

            let mut inode = INode {
                hash: hash.clone(),
                ino: self.inodes.len() as u64,
                parent_hash: block.prev_hash,
                block: data,
                timestamp: block.timestamp,
                block_client: self.block_client.clone(),
                middleware_client: self.middleware_client.clone(),
                journal: HashMap::new(),
                prev_data_hash: self.data_root.clone()
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