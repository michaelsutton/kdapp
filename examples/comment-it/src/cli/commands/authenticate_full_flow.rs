use clap::Args;
use std::error::Error;
use secp256k1::Keypair;
use crate::wallet::get_wallet_for_command;

// Import the auth functions from the auth module
use crate::auth::{run_http_coordinated_authentication, run_session_revocation, AuthenticationResult};

#[derive(Args)]
pub struct AuthenticateFullFlowCommand {
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    pub peer: String,
    
    #[arg(long, default_value = "10")]
    pub session_duration: u64,
    
    #[arg(long, default_value = "30")]
    pub auth_timeout: u64,
    
    #[arg(short, long)]
    pub key: Option<String>,
}

impl AuthenticateFullFlowCommand {
    pub async fn execute(self) -> Result<(), Box<dyn Error>> {
        println!("🔄 Starting complete authentication lifecycle test");
        println!("⏱️  Auth timeout: {}s, Session duration: {}s", self.auth_timeout, self.session_duration);
        println!("🎯 Organizer peer: {}", self.peer);
        
        // Get wallets for both funding and auth
        let funding_wallet = get_wallet_for_command("participant-peer", self.key.as_deref())?;
        let auth_wallet = get_wallet_for_command("authenticate", None)?;
        
        run_full_authentication_cycle(
            funding_wallet.keypair, 
            auth_wallet.keypair, 
            self.peer, 
            self.session_duration, 
            self.auth_timeout
        ).await?;
        
        Ok(())
    }
}

// Moved from main.rs - the complete authentication lifecycle implementation
async fn run_full_authentication_cycle(
    funding_keypair: Keypair,
    auth_keypair: Keypair, 
    peer_url: String,
    session_duration: u64,
    auth_timeout: u64
) -> Result<(), Box<dyn Error>> {
    println!("🔄 Starting complete authentication lifecycle test");
    println!("⏱️  Phase 1: Login ({}s timeout)", auth_timeout);
    
    // Phase 1: Authenticate with timeout
    let auth_timeout_duration = tokio::time::Duration::from_secs(auth_timeout);
    let auth_future = run_http_coordinated_authentication(funding_keypair, auth_keypair, peer_url.clone());
    
    let auth_result = tokio::time::timeout(auth_timeout_duration, auth_future).await;
    
    let authentication_details: AuthenticationResult = match auth_result {
        Ok(Ok(auth_details)) => {
            println!("✅ Phase 1: Authentication successful!");
            println!("📋 Episode ID: {}, Session Token: {}", auth_details.episode_id, auth_details.session_token);
            auth_details
        }
        Ok(Err(e)) => {
            println!("❌ Phase 1: Authentication failed: {}", e);
            return Err(e);
        }
        Err(_) => {
            println!("⏰ Phase 1: Authentication timed out after {}s", auth_timeout);
            return Err("Authentication timeout".into());
        }
    };
    
    // Phase 2: Simulate active session
    println!("⏱️  Phase 2: Active session ({}s duration)", session_duration);
    println!("🔒 Session is active - simulating user activity...");
    
    tokio::time::sleep(tokio::time::Duration::from_secs(session_duration)).await;
    
    // Phase 3: Logout using authentication details from Phase 1
    println!("⏱️  Phase 3: Logout initiated");
    println!("🚪 Revoking session {} for episode {}", authentication_details.session_token, authentication_details.episode_id);
    
    match run_session_revocation(auth_keypair, authentication_details.episode_id, authentication_details.session_token, peer_url).await {
        Ok(_) => {
            println!("✅ Phase 3: Session revocation successful!");
            println!("✅ Full authentication cycle test completed - Login → Active Session → Logout");
        }
        Err(e) => {
            println!("❌ Phase 3: Session revocation failed: {}", e);
            println!("⚠️  Authentication cycle incomplete - logout failed");
            return Err(format!("Logout failed: {}", e).into());
        }
    }
    
    Ok(())
}