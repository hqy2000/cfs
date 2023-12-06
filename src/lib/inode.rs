use std::collections::HashMap;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use fuser::{FileAttr, FileType};
use fuser::FileType::Directory;
use futures::executor::block_on;
use futures::future::join_all;
use log::debug;

use crate::client::{BlockClient, FSMiddlewareClient};
use crate::fs::BLOCK_SIZE;
use crate::proto::block::{DataBlock, DataCapsuleFileSystemBlock, INodeBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

#[derive(Clone)]
pub struct INode {
    pub hash: String,
    pub ino: u64,
    pub parent_hash: String,
    pub block: INodeBlock,
    pub timestamp: i64,
    pub block_client: Arc<BlockClient>,
    pub middleware_client: Option<Arc<FSMiddlewareClient>>,
    pub journal: HashMap<String, Vec<u8>>,
    pub prev_data_hash: String
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

impl INode {
    async fn read_block(&self, idx: usize, offset: u64) -> Vec<u8> {
        return if let Some(hash) = self.block.hashes.get(idx) {
            if self.journal.contains_key(hash) {
                let block = self.journal.get(hash).cloned().unwrap();
                block[offset as usize..].to_vec()
            } else {
                let response = self.block_client.get_block(hash.clone()).await.unwrap();
                response[offset as usize..].to_vec()
            }
        } else {
            vec![]
        }
    }

    async fn write_block(&self, uid: u32, data: Vec<u8>) -> String {
        let data = DataBlock { data };

        let block = DataCapsuleFileSystemBlock {
            prev_hash: self.prev_data_hash.to_string(),
            block: Some(Block::Data(data)),
            updated_by: Some(self.middleware_client.clone().unwrap().get_id(uid as u64)),
            signature: vec![],
        };

        self.middleware_client.clone().unwrap().put_data(block, self.parent_hash.clone()).await.unwrap().hash.unwrap()
    }

    pub fn read(&mut self, offset: i64, size: u32) -> Vec<u8> {
        let mut current = (offset / BLOCK_SIZE) as usize;
        let mut total_read_bytes = 0;
        let mut data = vec![];

        // read partial block first
        debug!("Getting partial block {} for offset {} size {}\n", current, offset, size);
        let mut blocks = vec![];

        let partial_block = self.read_block(current, (offset % BLOCK_SIZE) as u64);
        blocks.push(partial_block);
        total_read_bytes += BLOCK_SIZE - (offset % BLOCK_SIZE);
        current += 1;

        // the until everything is read
        while total_read_bytes < size as i64 {
            debug!("Getting full {} block for offset {} size {}\n", current, offset, size);
            let block = self.read_block(current, 0);
            blocks.push(block);
            total_read_bytes += BLOCK_SIZE;
            current += 1;
        }

        let results = block_on(async {
            join_all(blocks).await
        });

        for block in results {
            data.extend_from_slice(&block);
        }

        return data;
    }

    pub fn write(&mut self, uid: u32, offset: i64, data: &[u8]) {
        let mut block_id = (offset / BLOCK_SIZE) as usize;
        while self.block.hashes.len() <= block_id { // 0 fill if offset is past EOF
            let hash = block_on(self.write_block(uid, vec![0u8; BLOCK_SIZE as usize]));
            self.block.hashes.push(hash);
        }

        let mut next = 0;
        // read partial block first
        debug!("Getting partial block {} for offset {}\n", block_id, offset);
        let mut block = block_on(self.read_block(block_id, 0));
        block.truncate((offset % BLOCK_SIZE) as usize);

        while next < data.len() {
            let remaining_bytes = BLOCK_SIZE as usize - block.len();
            if data.len() >= next + remaining_bytes { // enough bytes to fill all
                block.extend_from_slice(&data[next..(next + remaining_bytes)]);
            } else { // need to check if we still have existing data
                block.extend_from_slice(&data[next..]);
                if block_id < self.block.hashes.len() {
                    debug!("Getting partial block {} for offset {}\n", block_id, offset);
                    let prev_block = block_on(self.read_block(block_id, block.len() as u64));
                    block.extend_from_slice(&prev_block)
                }
            }

            if block.len() != BLOCK_SIZE as usize {
                debug!("Fill block with 0s");
                block.extend_from_slice(&vec![0u8; BLOCK_SIZE as usize - block.len()]);
            }

            debug!("Publish block len = {}", block.len());
            let hash = block_on(self.write_block(uid, block));

            if block_id < self.block.hashes.len() { // within bounds, replace existing
                self.block.hashes[block_id] = hash;
            } else {
                self.block.hashes.push(hash);
            }

            next += remaining_bytes;
            block_id += 1;
            block = vec![];
        }
    }

    // pub fn write(&mut self, uid: u32, offset: i64, data: &[u8]) {
    //     let mut block_id = (offset / BLOCK_SIZE) as usize;
    //
    //     let mut to_join = vec![];
    //     while self.block.hashes.len() + to_join.len() <= block_id { // 0 fill if offset is past EOF
    //         to_join.push(self.write_block(uid, vec![0u8; BLOCK_SIZE as usize]));
    //     }
    //     let new_hashes = block_on(join_all(to_join));
    //     self.block.hashes.extend(new_hashes);
    //
    //     let mut next = 0;
    //     let mut to_join = vec![];
    //     // read partial block first
    //     debug!("Getting partial block {} for offset {}\n", block_id, 0);
    //     let mut block = block_on(self.read_block(block_id, 0));
    //     block.truncate((offset % BLOCK_SIZE) as usize);
    //
    //     while next < data.len() {
    //         let remaining_bytes = BLOCK_SIZE as usize - block.len();
    //         if data.len() >= next + remaining_bytes { // enough bytes to fill all
    //             block.extend_from_slice(&data[next..(next + remaining_bytes)]);
    //         } else { // need to check if we still have existing data
    //             block.extend_from_slice(&data[next..]);
    //             if block_id < self.block.hashes.len() {
    //                 debug!("Getting partial block {} for offset {}\n", block_id, block.len());
    //                 let prev_block = block_on(self.read_block(block_id, block.len() as u64));
    //                 block.extend_from_slice(&prev_block)
    //             }
    //         }
    //
    //         if block.len() != BLOCK_SIZE as usize {
    //             debug!("Fill block with 0s");
    //             block.extend_from_slice(&vec![0u8; BLOCK_SIZE as usize - block.len()]);
    //         }
    //
    //         debug!("Publish block len = {}", block.len());
    //         to_join.push(self.write_block(uid, block));
    //
    //         next += remaining_bytes;
    //         block_id += 1;
    //         block = vec![];
    //     }
    //
    //     let new_hashes = block_on(join_all(to_join));
    //     let block_id = (offset / BLOCK_SIZE) as usize;
    //     self.block.hashes.truncate(block_id);
    //     self.block.hashes.extend(new_hashes);
    // }
}