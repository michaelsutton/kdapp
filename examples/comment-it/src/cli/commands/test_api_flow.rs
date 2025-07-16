use clap::Args;
use reqwest::Client;
use serde_json::Value;
use secp256k1::{Keypair, Secp256k1, SecretKey};
use kdapp::pki::{sign_message, to_message};

#[derive(Args)]
pub struct TestApiFlowCommand {
    #[arg(short, long, default_value = "http://127.0.0.1:8080")]
    pub peer: String,
}

impl TestApiFlowCommand {
    pub async fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting API Flow Test against coordination peer: {}", self.peer);
        let client = Client::new();
        
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let keypair = Keypair::from_secret_key(&secp, &secret_key);
        let public_key_hex = hex::encode(keypair.public_key().serialize());

        println!("🔑 Generated temporary client keypair. Public key: {}", public_key_hex);

        // Step 1: Start Auth
        println!("
[1/5] Calling POST /auth/start...");
        let start_res = client
            .post(format!("{}/auth/start", self.peer))
            .json(&serde_json::json!({ "public_key": public_key_hex }))
            .send()
            .await?;

        if !start_res.status().is_success() {
            return Err(format!("Failed to start auth: {}", start_res.status()).into());
        }
        let start_data: Value = start_res.json().await?;
        let episode_id = start_data["episode_id"].as_u64().unwrap();
        println!("✅ Success! Episode ID: {}", episode_id);

        // Step 2: Request Challenge
        println!("
[2/5] Calling POST /auth/request-challenge...");
        let req_challenge_res = client
            .post(format!("{}/auth/request-challenge", self.peer))
            .json(&serde_json::json!({ "episode_id": episode_id, "public_key": public_key_hex }))
            .send()
            .await?;
        if !req_challenge_res.status().is_success() {
            return Err(format!("Failed to request challenge: {}", req_challenge_res.status()).into());
        }
        println!("✅ Success! Challenge request sent.");

        // Step 3: Poll for Challenge
        println!("
[3/5] Polling GET /auth/status/{} for challenge...", episode_id);
        let mut challenge = String::new();
        for _ in 0..10 {
            let status_res = client.get(format!("{}/auth/status/{}", self.peer, episode_id)).send().await?;
            let status_data: Value = status_res.json().await?;
            if let Some(c) = status_data["challenge"].as_str() {
                challenge = c.to_string();
                println!("✅ Success! Received challenge: {}", challenge);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        if challenge.is_empty() {
            return Err("Timeout waiting for challenge".into());
        }

        // Step 4: Sign Challenge
        println!("
[4/5] Signing challenge locally...");
        let msg = to_message(&challenge);
        let signature = sign_message(&keypair.secret_key(), &msg);
        let signature_hex = hex::encode(signature.0.serialize_der());
        println!("✅ Challenge signed.");

        // Step 5: Verify Auth
        println!("
[5/5] Calling POST /auth/verify...");
        let verify_res = client
            .post(format!("{}/auth/verify", self.peer))
            .json(&serde_json::json!({
                "episode_id": episode_id,
                "signature": signature_hex,
                "nonce": challenge
            }))
            .send()
            .await?;
        if !verify_res.status().is_success() {
            return Err(format!("Failed to verify auth: {}", verify_res.status()).into());
        }
        let verify_data: Value = verify_res.json().await?;
        println!("✅ Verification request successful: {}", verify_data);

        println!("
🏁 Verification complete! Checking final status...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let final_status_res = client.get(format!("{}/auth/status/{}", self.peer, episode_id)).send().await?;
        let final_status_data: Value = final_status_res.json().await?;

        println!("
--- FINAL RESULT ---");
        println!("{}", serde_json::to_string_pretty(&final_status_data)?);
        println!("--------------------");

        if final_status_data["authenticated"].as_bool().unwrap_or(false) {
            println!("🎉🎉🎉 API Flow Test Successful! 🎉🎉🎉");
        } else {
            println!("❌❌❌ API Flow Test Failed! ❌❌❌");
        }

        Ok(())
    }
}