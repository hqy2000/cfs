use std::cmp::min;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

use fuser::{Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEntry, ReplyWrite, Request};
use libc::ENOENT;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::Sha256;

use crate::inode_cache::{INode, INodeCache};
use crate::client::{BlockClient, FSMiddlewareClient};
use crate::crypto::SignableBlock;
use crate::proto::block::{DataBlock, DataCapsuleFileSystemBlock, Id, INodeBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

const TTL: Duration = Duration::from_secs(1); // 1 second
const BLOCK_SIZE: i64 = 512;

pub struct DCFS2 {
    pub block_client: BlockClient,
    pub inode_cache: INodeCache,
    pub middleware_client: Option<FSMiddlewareClient>,
    pub signing_key: Option<SigningKey<Sha256>>,
}


impl Filesystem for DCFS2 {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent < self.inode_cache.num_inodes() {
            let sub_inodes = self.inode_cache.get_sub_inodes(parent);
            if let Some(inode) = sub_inodes.iter().find(|x| x.block.filename == name.to_str().unwrap().as_bytes()) {
                reply.entry(&TTL, &inode.to_file_attr(), 0);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino < self.inode_cache.num_inodes() {
            reply.attr(&TTL, &self.inode_cache.get_inode(ino).to_file_attr());
        } else {
            reply.error(ENOENT)
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino < self.inode_cache.num_inodes() {
            let inode = self.inode_cache.get_inode(ino);

            let mut current = offset;
            let mut data = vec![];

            while current < offset + i64::from(size) {
                if let Some(hash) = inode.block.hashes.get((current / BLOCK_SIZE) as usize) {
                    print!("Getting {} block ({}) for offset {} size {}\n", current / BLOCK_SIZE, hash, offset, size);
                    let response = self.block_client.get_block(hash).unwrap();
                    data.extend_from_slice(&response[(current % BLOCK_SIZE) as usize..]);
                    current = (current + BLOCK_SIZE) / BLOCK_SIZE * BLOCK_SIZE;
                } else {
                    break;
                }
            }
            reply.data(&data);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino >= self.inode_cache.num_inodes() {
            reply.error(ENOENT);
            return;
        }

        let inode = self.inode_cache.get_inode(ino);
        if inode.block.kind != Kind::Directory.into() {
            reply.error(ENOENT);
            return;
        }

        let mut children = self.inode_cache.get_sub_inodes(ino);
        let parent_inode = self.inode_cache.get_inode(inode.parent_ino);

        children.insert(0, INode{
            hash: parent_inode.hash,
            ino: parent_inode.ino,
            parent_ino: parent_inode.parent_ino,
            block: INodeBlock {
                filename: "..".into(),
                size: 0,
                kind: parent_inode.block.kind,
                hashes: parent_inode.block.hashes,
                write_allow_list: vec![]
            },
            timestamp: 0
        });

        children.insert(0, INode{
            hash: inode.hash,
            ino: inode.ino,
            parent_ino: inode.parent_ino,
            block: INodeBlock {
                filename: ".".into(),
                size: 0,
                kind: inode.block.kind,
                hashes: inode.block.hashes,
                write_allow_list: vec![]
            },
            timestamp: 0
        });

        for (i, entry) in children.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.ino, (i + 1) as i64, entry.get_file_type(),
                         OsStr::from_bytes(&entry.block.filename)) {
                break;
            }
        }
        reply.ok();
    }

    fn create(
        &mut self,
        req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let parent_block = self.inode_cache.get_inode(parent);

        let mut id = Id {
            pub_key: Vec::from(include_str!("../../key/client1_public.pem")), // todo: store this along key.
            uid: req.uid() as u64,
            signature: vec![],
        };
        id.sign(self.signing_key.as_ref().unwrap());

        let inode_block = INodeBlock {
            filename: Vec::from(name.to_str().unwrap()),
            size: 0,
            kind: Kind::RegularFile.into(),
            hashes: vec![],
            write_allow_list: parent_block.block.write_allow_list.clone(),
        };

        let mut block = DataCapsuleFileSystemBlock {
            prev_hash: parent_block.hash.clone(),
            block: Some(Block::Inode(inode_block)),
            updated_by: Some(id),
            signature: vec![],
        };
        block.sign(self.signing_key.as_ref().unwrap());

        let response = self.middleware_client.as_mut().unwrap().put_inode(block).unwrap();
        self.inode_cache.resolve(response.clone());

        reply.created(&TTL, &self.inode_cache.get_inode(self.inode_cache.get_ino(response.clone())).to_file_attr(), 0, 100, 0); // todo: file handle?
    }

    fn write(
        &mut self,
        req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        if ino >= self.inode_cache.num_inodes() {
            reply.error(ENOENT);
            return;
        }

        let inode = self.inode_cache.get_inode(ino);
        if inode.block.kind != Kind::RegularFile.into() {
            reply.error(ENOENT);
            return;
        }

        let mut updatedBy = Id {
            pub_key: Vec::from(include_str!("../../key/client1_public.pem")), // todo: store this along key.
            uid: req.uid() as u64,
            signature: vec![],
        };
        updatedBy.sign(self.signing_key.as_ref().unwrap());

        let mut hashes: Vec<String> = vec![];

        let mut id = 0;


        while id * BLOCK_SIZE < offset + data.len() as i64 {
            let lower = id * BLOCK_SIZE;
            let upper = (id + 1) * BLOCK_SIZE;

            if upper < offset {
                hashes.push(inode.block.hashes[id as usize].clone())
            } else {
                let mut block_data = vec![];

                if lower < offset {
                    if let Some(hash) = inode.block.hashes.get(id as usize) {
                        let response = self.block_client.get_block(hash).unwrap();
                        block_data = response;
                    }
                    block_data.truncate((offset % BLOCK_SIZE) as usize);
                    block_data.extend_from_slice(&data[..min((BLOCK_SIZE - (offset % BLOCK_SIZE)) as usize, data.len())])
                } else {
                    block_data = data[(lower - offset) as usize..min((upper - offset) as usize, data.len())].to_owned();
                }

                let data = DataBlock {
                    data: block_data
                };

                let mut block = DataCapsuleFileSystemBlock {
                    prev_hash: "file_hash1".into(),  // always file_hash1 because we don't care about the structure
                    block: Some(Block::Data(data)),
                    updated_by: Some(updatedBy.clone()),
                    signature: vec![],
                };
                block.sign(self.signing_key.as_ref().unwrap());

                let response = self.middleware_client.as_mut().unwrap().put_data(block, inode.hash.clone()).unwrap();
                hashes.push(response);
            }

            id += 1
        }

        let mut block = inode.block.clone();
        block.hashes = hashes;
        block.size = (offset + data.len() as i64) as u64;
        let mut block = DataCapsuleFileSystemBlock {
            prev_hash: self.inode_cache.get_inode(inode.parent_ino).hash,
            block: Some(Block::Inode(block)),
            updated_by: Some(updatedBy),
            signature: vec![],
        };
        block.sign(self.signing_key.as_ref().unwrap());

        let response = self.middleware_client.as_mut().unwrap().put_inode(block).unwrap();
        self.inode_cache.resolve(response.clone());

        reply.written(data.len() as u32);
    }


}

