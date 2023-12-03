use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, SystemTime};

use fuser::{Filesystem, FileType, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyWrite, Request, TimeOrNow};
use fuser::FileType::{Directory, RegularFile};
use libc::ENOENT;

use crate::cache::Cache;
use crate::proto::block::i_node_block::Kind;

const TTL: Duration = Duration::from_secs(1); // 1 second
pub const BLOCK_SIZE: i64 = 512;

pub struct DCFS2 {
    pub cache: Cache,
}

impl DCFS2 {
    fn _delete(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, file_type: FileType, reply: ReplyEmpty) {
        let node = self.cache.find_child_node(parent, name);
        if let Some(node) = node {
            if node.get_file_type() != file_type {
                reply.error(ENOENT);
            } else {
                self.cache.delete(req.uid(), node.ino);
                reply.ok();
            }
        } else {
            reply.error(ENOENT);
        }
    }
}


impl Filesystem for DCFS2 {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if let Some(inode) = self.cache.find_child_node(parent, name) {
            reply.entry(&TTL, &inode.to_file_attr(), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        if ino < self.cache.num_inodes() {
            reply.attr(&TTL, &self.cache.get_inode(ino).0.to_file_attr());
        } else {
            reply.error(ENOENT)
        }
    }

    fn setattr(&mut self, req: &Request<'_>, ino: u64, _mode: Option<u32>, _uid: Option<u32>, _gid: Option<u32>, size: Option<u64>, _atime: Option<TimeOrNow>, _mtime: Option<TimeOrNow>, _ctime: Option<SystemTime>, _fh: Option<u64>, _crtime: Option<SystemTime>, _chgtime: Option<SystemTime>, _bkuptime: Option<SystemTime>, _flags: Option<u32>, reply: ReplyAttr) {
        if let Some(size) = size {
            let mut block = self.cache.get_inode(ino).0.block;
            block.size = size;
            self.cache.update(req.uid(), ino, block);
        }
        reply.attr(&TTL, &self.cache.get_inode(ino).0.to_file_attr())
    }

    fn mkdir(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, _mode: u32, _umask: u32, reply: ReplyEntry) {
        self.cache.create(req.uid(), parent, name, Kind::Directory);

        reply.entry(&TTL, &self.cache.get_inode(self.cache.find_child_node(parent, name).unwrap().ino).0.to_file_attr(), 0);
    }

    fn unlink(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        self._delete(req, parent, name, RegularFile, reply);
    }

    fn rmdir(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        self._delete(req, parent, name, Directory, reply);
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, _flags: i32, _lock: Option<u64>, reply: ReplyData) {
        let mut file_view = self.cache.get_file_view(ino);
        reply.data(&file_view.read(offset, size));
    }

    fn write(&mut self, req: &Request<'_>, ino: u64, _fh: u64, offset: i64, data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: Option<u64>, reply: ReplyWrite) {
        let inode = self.cache.get_inode(ino).0;
        if inode.block.kind != Kind::RegularFile.into() {
            reply.error(ENOENT);
            return;
        }
        let mut file_view = self.cache.get_file_view(ino);
        file_view.write(req.uid(), offset, data);

        let mut block = inode.block.clone();
        block.hashes = file_view.hashes.clone();
        block.size = (offset + data.len() as i64) as u64;
        self.cache.update(req.uid(), ino, block);
        reply.written(data.len() as u32);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let mut inode = self.cache.get_inode(ino).0;
        if inode.get_file_type() != Directory {
            reply.error(ENOENT);
            return;
        }

        let mut children = self.cache.get_inode(ino).1;

        let mut parent_inode = self.cache.get_inode(inode.parent_ino).0;
        parent_inode.block.filename = Vec::from("..");
        children.insert(0, parent_inode);
        inode.block.filename = Vec::from(".");
        children.insert(0, inode);

        for (i, entry) in children.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if entry.is_deleted() {continue;}
            if reply.add(entry.ino, (i + 1) as i64, entry.get_file_type(), OsStr::from_bytes(&entry.block.filename)) {
                break;
            }
        }
        reply.ok();
    }

    fn create(&mut self, req: &Request<'_>, parent: u64, name: &OsStr, _mode: u32, _umask: u32, _flags: i32, reply: ReplyCreate) {
        self.cache.create(req.uid(), parent, name, Kind::RegularFile);
        reply.created(&TTL, &self.cache.get_inode(self.cache.find_child_node(parent, name).unwrap().ino).0.to_file_attr(), 0, 100, 0); // todo: file handle?
    }
}

