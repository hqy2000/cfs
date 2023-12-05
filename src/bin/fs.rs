use clap::{Arg, Command};
use fuser::MountOption;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use tonic::transport::{Certificate, ClientTlsConfig};

use lib::cache::Cache;
use lib::client::{BlockClient, FSMiddlewareClient, INodeClient};
use lib::fs::DCFS2;

fn main() {
    env_logger::init();
    let client1_signing_key = pkcs1v15::SigningKey::<Sha256>::from_pkcs8_pem(include_str!("../../key/client1_private.pem")).unwrap();

    let matches = Command::new("hello")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Act as a client, and mount FUSE at given path"),
        ).get_matches();
    // env_logger::init();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();
    let options = vec![
        MountOption::RW, // RO or RW
        MountOption::DefaultPermissions, // Use kernel to enforce permissions
        // MountOption::AllowRoot,
        // MountOption::AutoUnmount,
        MountOption::FSName("hello".to_string()) // todo: what's this?
    ];

    let ca = Certificate::from_pem(include_str!("../../key/loopback.hqy.moe_chain.pem"));
    let tls_config = ClientTlsConfig::new().ca_certificate(ca);

    fuser::mount2(DCFS2{
        cache: Cache::new(
            INodeClient::connect("https://loopback.hqy.moe:50052", tls_config.clone(), 5),
            BlockClient::connect("https://loopback.hqy.moe:50051", tls_config.clone(), 100),
            Some(FSMiddlewareClient::connect("https://loopback.hqy.moe:50060", tls_config,
                                             include_str!("../../key/client1_public.pem").parse().unwrap(),
            client1_signing_key)),
            "root".into(),
        ),
    }, mountpoint, &options).unwrap();
}