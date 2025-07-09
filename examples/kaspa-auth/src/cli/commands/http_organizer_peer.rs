use clap::Args;
use secp256k1::Keypair;
use std::error::Error;
use crate::api::http::organizer_peer::run_http_peer;

#[derive(Args)]
pub struct HttpOrganizerPeerCommand {
    #[arg(short, long, default_value = "8080")]
    pub port: u16,
    
    #[arg(short, long)]
    pub key: Option<String>,
}

impl HttpOrganizerPeerCommand {
    pub async fn execute(self) -> Result<(), Box<dyn Error>> {
        let provided_private_key = self.key.as_deref();
        run_http_peer(provided_private_key, self.port).await
    }
}



