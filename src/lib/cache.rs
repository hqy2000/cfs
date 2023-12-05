use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use fuser::{FileAttr, FileType};
use fuser::FileType::{Directory, RegularFile};
use log::debug;

use crate::client::{BlockClient, FSMiddlewareClient, INodeClient};
use crate::fs::BLOCK_SIZE;
use crate::proto::block::{DataBlock, DataCapsuleFileSystemBlock, INodeBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

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
            blocks: (self.block.size + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64, // round up
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
            blksize: BLOCK_SIZE as u32,
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
            0o700
        } else {
            0o700
        }
    }

    pub fn is_deleted(&self) -> bool {
        return self.block.kind == Kind::DeletedFolder.into() || self.block.kind == Kind::DeletedRegularFile.into();
    }
}

pub struct FileView {
    inode_hash: String,
    pub hashes: Vec<String>,
    block_client: Arc<Mutex<BlockClient>>,
    middleware_client: Option<Arc<Mutex<FSMiddlewareClient>>>
}

impl FileView {
    fn read_block(&mut self, idx: usize, offset: u64) -> Vec<u8> {
        return if let Some(hash) = self.hashes.get(idx) {
            let response = self.block_client.lock().unwrap().get_block(&hash).unwrap();
            response[offset as usize..].to_vec()
        } else {
            vec![]
        }
    }

    fn write_block(&mut self, uid: u32, data: Vec<u8>) -> String {
        let data = DataBlock { data };

        let block = DataCapsuleFileSystemBlock {
            prev_hash: "file_hash1".into(),  // TODO: change this; for now always file_hash1 because we don't care about the structure
            block: Some(Block::Data(data)),
            updated_by: Some(self.middleware_client.clone().unwrap().lock().unwrap().get_id(uid as u64)),
            signature: vec![],
        };

        let response = self.middleware_client.clone().unwrap().lock().unwrap().put_data(block, self.inode_hash.clone()).unwrap();
        return response;
    }

    pub fn read(&mut self, offset: i64, size: u32) -> Vec<u8> {
        let mut current = (offset / BLOCK_SIZE) as usize;
        let mut data = vec![];

        // read partial block first
        debug!("Getting partial block {} for offset {} size {}\n", current, offset, size);
        let block = self.read_block(current, (offset % BLOCK_SIZE) as u64);
        data.extend_from_slice(&block);
        current += 1;

        // the until everything is read
        while data.len() < size as usize {
            debug!("Getting full {} block for offset {} size {}\n", current, offset, size);
            let block = self.read_block(current, 0);
            data.extend_from_slice(&block);
            current += 1;

            if block.len() == 0 {
                break; // nothing to read anymore, break
            }
        }

        return data;
    }

    pub fn write(&mut self, uid: u32, offset: i64, data: &[u8]) {
        let mut block_id = (offset / BLOCK_SIZE) as usize;
        while self.hashes.len() <= block_id { // 0 fill if offset is past EOF
            let hash = self.write_block(uid, vec![0u8; BLOCK_SIZE as usize]);
            self.hashes.push(hash);
        }

        let mut next = 0;


        // read partial block first
        debug!("Getting partial block {} for offset {}\n", block_id, offset);
        let mut block = self.read_block(block_id, 0);
        block.truncate((offset % BLOCK_SIZE) as usize);

        while next < data.len() {
            let remaining_bytes = BLOCK_SIZE as usize - block.len();
            if data.len() >= next + remaining_bytes { // enough bytes to fill all
                block.extend_from_slice(&data[next..(next + remaining_bytes)]);
            } else { // need to check if we still have existing data
                block.extend_from_slice(&data[next..]);
                if block_id < self.hashes.len() {
                    debug!("Getting partial block {} for offset {}\n", block_id, offset);
                    let prev_block = self.read_block(block_id, block.len() as u64);
                    block.extend_from_slice(&prev_block)
                }
            }

            if block.len() != BLOCK_SIZE as usize {
                debug!("Fill block with 0s");
                block.extend_from_slice(&vec![0u8; BLOCK_SIZE as usize - block.len()]);
            }

            debug!("Publish block len = {}", block.len());
            let hash = self.write_block(uid, block);

            if block_id < self.hashes.len() { // within bounds, replace existing
                self.hashes[block_id] = hash;
            } else {
                self.hashes.push(hash);
            }

            next += remaining_bytes;
            block_id += 1;
            block = vec![];
        }
    }
}

pub struct Cache {
    inode_client: INodeClient,
    block_client: Arc<Mutex<BlockClient>>,
    middleware_client: Option<Arc<Mutex<FSMiddlewareClient>>>,
    inodes: Vec<(INode, Vec<INode>)>, // Vec<Node, Children>
    hash_to_ino: HashMap<String, u64> // Hash -> INode.ino
}

impl Cache {
    pub fn new(
        client: INodeClient,
        block_client: BlockClient,
        middleware: Option<FSMiddlewareClient>,
        root: String) -> Cache {
        let mut cache = Cache {
            inode_client: client,
            block_client: Arc::new(Mutex::new(block_client)),
            middleware_client: None,
            inodes: Vec::new(),
            hash_to_ino: HashMap::new()
        };
        if let Some(middleware) = middleware {
            cache.middleware_client = Some(Arc::new(Mutex::new(middleware)));
        }
        cache.build(root);
        return cache;
    }

    fn build(&mut self, root: String) {
        let block = self.inode_client.get(&root).unwrap();
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
            prev_hash: self.get_inode(self.get_inode(ino).0.parent_ino).0.hash,
            block: Some(Block::Inode(block)),
            updated_by: Some(self.middleware_client.clone().unwrap().lock().unwrap().get_id(uid as u64)),
            signature: vec![],
        };
        let hash = self.middleware_client.clone().unwrap().lock().unwrap().put_inode(block).unwrap();
        self.resolve(hash);
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
            updated_by: Some(self.middleware_client.clone().unwrap().lock().unwrap().get_id(uid as u64)),
            signature: vec![],
        };
        // block.sign(self.signing_key.as_ref().unwrap());
        let hash = self.middleware_client.clone().unwrap().lock().unwrap().put_inode(block).unwrap();
        self.resolve(hash);
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

    pub fn get_file_view(&self, ino: u64) -> FileView {
        let block = self.get_inode(ino).0;
        if block.get_file_type() != RegularFile {
            panic!("file view cannot be obtained for folders.")
        }

        return FileView{
            inode_hash: block.hash,
            hashes: block.block.hashes.clone(),
            block_client: self.block_client.clone(),
            middleware_client: self.middleware_client.clone()
        }
    }

    fn resolve(&mut self, hash: String) {
        if !self.hash_to_ino.contains_key(&hash) {
            let block = self.inode_client.get(&hash).unwrap();

            if !self.hash_to_ino.contains_key(&block.prev_hash) {
                self.resolve(block.prev_hash.clone());
            }

            if let Block::Inode(data) = block.fs.unwrap().block.unwrap() {
                let parent_ino = self.hash_to_ino.get(&block.prev_hash).unwrap();


                let mut inode = INode {
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