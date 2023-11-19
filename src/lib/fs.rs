use std::error::Error;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

use fuser::{Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEntry, Request};
use libc::ENOENT;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::Sha256;

use crate::cache::{INode, INodeCache};
use crate::client::{BlockClient, FSMiddlewareClient};
use crate::crypto::SignableBlock;
use crate::proto::block::{DataCapsuleFileSystemBlock, Id, INodeBlock};
use crate::proto::block::data_capsule_file_system_block::Block;
use crate::proto::block::i_node_block::Kind;

const TTL: Duration = Duration::from_secs(1); // 1 second

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
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino < self.inode_cache.num_inodes() {
            let inode = self.inode_cache.get_inode(ino);

            // todo: multiple blocks
            let response: Result<Vec<u8>, Box<dyn Error>> = self.block_client.get_block(inode.block.hashes.get(0).unwrap());
            reply.data(&response.unwrap()[offset as usize..]);
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
            }
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
            }
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

        reply.created(&TTL, &self.inode_cache.get_inode(self.inode_cache.get_ino(response.clone())).to_file_attr(), 0, 1, 0); // todo: file handle?
    }

}

