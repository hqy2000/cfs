use std::fs;
use std::sync::Arc;

use clap::{Arg, Command};
use config::{Config, ConfigError, File};
use futures::future::join_all;
use prost::Message;
use rsa::pkcs1v15;
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::Sha256;
use serde::Deserialize;
use tokio::sync::Mutex;
use tonic::{
    transport::{
        Identity, Server, ServerTlsConfig,
    },
};

use lib::proto::data_capsule::data_capsule_server::DataCapsuleServer;
use lib::proto::data_capsule::DataCapsuleServerData;
use lib::server::MyDataCapsule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("server")
        .arg(
            Arg::new("CONFIG_FILE")
                .required(true)
                .index(1)
                .help("Configuration file of the server.")
        ).get_matches();

    let config = ServersConfig::new(matches.get_one::<String>("CONFIG_FILE").unwrap()).unwrap();
    let mut v = Vec::new();

    for server in config.servers {
        let identity = Identity::from_pem(
            fs::read(server.tls.certificate).unwrap(),
            fs::read(server.tls.private_key).unwrap(),
        );
        let data_capsule_addr = format!("{}:{}", server.address, server.port).parse()?;
        let data_capsule = MyDataCapsule {
            data: Arc::new(Mutex::new(DataCapsuleServerData::decode(fs::read(server.data_file).unwrap().as_slice()).unwrap())),
            verifying_key: pkcs1v15::VerifyingKey::<Sha256>::read_public_key_pem_file(server.verifying_key).unwrap()
        };
        v.push(Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity.clone()))?
            .add_service(DataCapsuleServer::new(data_capsule))
            .serve(data_capsule_addr)
        );
        println!("Listening {}:{}", server.address, server.port);
    }

    join_all(v).await; // TODO: panic() messages will not show up
    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct TLS {
    certificate: String,
    private_key: String
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct ServerConfig {
    is_crypto_enabled: bool,
    address: String,
    port: u16,
    data_file: String,
    tls: TLS,
    verifying_key: String
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
struct ServersConfig {
    servers: Vec<ServerConfig>
}

impl ServersConfig {
    pub fn new(config_file: &String) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(config_file))
            .build()?;
        s.try_deserialize()
    }
}