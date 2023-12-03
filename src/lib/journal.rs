use std::collections::VecDeque;
use crate::client::{FSMiddlewareClient};
use crate::proto::block::DataCapsuleFileSystemBlock;

pub struct Journal {
    pub middleware: FSMiddlewareClient,
    pub queue: VecDeque<DataCapsuleFileSystemBlock>
}

impl Journal {

}