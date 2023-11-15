use clap::{Arg, ArgAction, Command};
use fuser::MountOption;
use lib::client::{BlockClient, INodeClient};
use lib::fs::DCFS2;
use lib::proto::block::data_capsule_block::Block;

fn main() {
    let matches = Command::new("hello")
        .author("Christopher Berner")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Act as a client, and mount FUSE at given path"),
        )
        .arg(
            Arg::new("auto_unmount")
                .long("auto_unmount")
                .action(ArgAction::SetTrue)
                .help("Automatically unmount on process exit"),
        )
        .arg(
            Arg::new("allow-root")
                .long("allow-root")
                .action(ArgAction::SetTrue)
                .help("Allow root user to access filesystem"),
        )
        .get_matches();
    // env_logger::init();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();
    let mut options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }
    fuser::mount2(DCFS2{
        block_client: BlockClient::connect("http://[::1]:50051"),
        inode_client: INodeClient::connect("http://[::1]:50052")
    }, mountpoint, &options).unwrap();
}