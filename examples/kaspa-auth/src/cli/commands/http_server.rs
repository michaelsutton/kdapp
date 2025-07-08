use clap::Args;
use secp256k1::Keypair;
use std::error::Error;
use crate::api::http::server::run_http_server;

#[derive(Args)]
pub struct HttpServerCommand {
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
    
    #[arg(short, long)]
    pub key: Option<String>,
}

impl HttpServerCommand {
    pub async fn execute(self) -> Result<(), Box<dyn Error>> {
        let provided_private_key = self.key.as_deref();
        run_http_server(provided_private_key, self.port).await
    }
}

fn parse_private_key(hex_str: &str) -> Result<Keypair, Box<dyn Error>> {
    use secp256k1::{Secp256k1, SecretKey};
    
    let secp = Secp256k1::new();
    let secret_bytes = hex::decode(hex_str)?;
    let secret_key = SecretKey::from_slice(&secret_bytes)?;
    Ok(Keypair::from_secret_key(&secp, &secret_key))
}

