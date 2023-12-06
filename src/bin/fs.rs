use std::fs;
use clap::{Arg, Command};
use config::{Config, ConfigError, File};
use fuser::MountOption;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use serde::{Deserialize, Serialize};
use tonic::transport::{Certificate, ClientTlsConfig};

use lib::cache::Cache;
use lib::client::{BlockClient, FSMiddlewareClient, INodeClient};
use lib::fs::CFS;

fn main() {
    env_logger::init();

    let matches = Command::new("fs")
        .arg(
            Arg::new("CONFIG_FILE")
                .required(true)
                .index(1)
                .help("Configuration"))
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(2)
                .help("Act as a client, and mount FUSE at given path")
        ).get_matches();

    let mut options = vec![
        MountOption::DefaultPermissions, // Use kernel to enforce permissions
        MountOption::FSName("cfs".into()),
    ];

    let config = ClientConfig::new(matches.get_one::<String>("CONFIG_FILE").unwrap()).unwrap();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();

    let ca = Certificate::from_pem(fs::read(config.tls.ca).unwrap());
    let tls_config = ClientTlsConfig::new().ca_certificate(ca);

    let middleware_client: Option<FSMiddlewareClient>;
    if let Some(middleware_config) = config.middleware {
        middleware_client = Some(FSMiddlewareClient::connect(
            &middleware_config.url, tls_config.clone(), fs::read_to_string(middleware_config.verifying_key).unwrap(),
            pkcs1v15::SigningKey::<Sha256>::read_pkcs8_pem_file(middleware_config.signing_key).unwrap()));
        options.push(MountOption::RW);
    } else {
        middleware_client = None;
        options.push(MountOption::RO);
    }

    fuser::mount2(CFS {
        cache: Cache::new(
            INodeClient::connect(config.inode_server.url.as_ref(), tls_config.clone(), config.inode_server.cache_size),
            BlockClient::connect(config.data_server.url.as_ref(), tls_config.clone(), config.data_server.cache_size),
            middleware_client, config.inode_server.root, config.data_server.root, config.block_size
        ),
    }, mountpoint, &options).unwrap();
}


#[derive(Serialize, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct TLS {
    pub ca: String,
}

#[derive(Serialize, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct Middleware {
    pub url: String,
    pub verifying_key: String,
    pub signing_key: String,
}

#[derive(Serialize, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct Server {
    pub url: String,
    pub cache_size: usize,
    pub root: String
}

#[derive(Serialize, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct ClientConfig {
    pub block_size: u16,
    pub is_crypto_enabled: bool,
    pub data_server: Server,
    pub inode_server: Server,
    pub middleware: Option<Middleware>,
    pub tls: TLS,
}

impl ClientConfig {
    pub fn new(config_file: &String) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(config_file))
            .build()?;
        s.try_deserialize()
    }
}