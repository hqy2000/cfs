use std::error::Error;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration};
use crate::client::{BlockClient, INodeClient};

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct DCFS2 {
    pub block_client: BlockClient,
    pub inode_client: INodeClient
}


impl Filesystem for DCFS2 {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == self.inode_client.hash_to_ino("root") && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &self.inode_client.get_inode("file").unwrap(), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino == self.inode_client.hash_to_ino("root") {
            reply.attr(&TTL, &self.inode_client.get_inode("root").unwrap());
        } else if ino == self.inode_client.hash_to_ino("file") {
            reply.attr(&TTL, &self.inode_client.get_inode("file").unwrap());
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
        if ino == self.inode_client.hash_to_ino("file") {
            let response: Result<Vec<u8>, Box<dyn Error>> = self.block_client.get_block("file_hash");
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
        if ino != self.inode_client.hash_to_ino("root") {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (self.inode_client.hash_to_ino("root"), FileType::Directory, "."),
            (self.inode_client.hash_to_ino("root"), FileType::Directory, ".."),
            (self.inode_client.hash_to_ino("file"), FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

