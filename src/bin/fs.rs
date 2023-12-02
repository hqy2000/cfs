use clap::{Arg, ArgAction, Command};
use fuser::MountOption;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use lib::inode_cache::INodeCache;
use lib::client::{BlockClient, FSMiddlewareClient, INodeClient};
use lib::fs::DCFS2;

fn main() {
    let client1_signing_key = pkcs1v15::SigningKey::<Sha256>::from_pkcs8_pem(include_str!("../../key/client1_private.pem")).unwrap();

    let matches = Command::new("hello")
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
    let mut options = vec![
        MountOption::RW, // RO or RW
        MountOption::FSName("hello".to_string()) // todo: what's this?
    ];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }
    fuser::mount2(DCFS2{
        block_client: BlockClient::connect("https://127.0.0.1:50056"),
        inode_cache: INodeCache::new(INodeClient::connect("https://127.0.0.1:50057"), "root".into()),
        middleware_client: Some(FSMiddlewareClient::connect("http://127.0.0.1:50065")),
        signing_key: Some(client1_signing_key)
    }, mountpoint, &options).unwrap();
}