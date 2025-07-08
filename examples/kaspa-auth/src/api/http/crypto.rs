// src/api/http/crypto.rs
use kdapp::pki::PubKey;
use secp256k1::{Message, Secp256k1, ecdsa::Signature};
use sha2::{Sha256, Digest};

pub fn parse_client_pubkey(pubkey_hex: &str) -> Result<PubKey, String> {
    let bytes = hex::decode(pubkey_hex).map_err(|_| "Invalid hex")?;
    if bytes.len() != 33 { return Err("Invalid length".to_string()); }
    let pk = secp256k1::PublicKey::from_slice(&bytes)
        .map_err(|_| "Invalid pubkey")?;
    Ok(PubKey(pk))
}

pub fn verify_signature(pubkey: &PubKey, message: &str, sig_hex: &str) -> Result<bool, String> {
    let sig_bytes = hex::decode(sig_hex).map_err(|_| "Invalid hex")?;
    if sig_bytes.len() != 64 { return Err("Invalid sig length".to_string()); }
    
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    let hash = hasher.finalize();
    
    let secp = Secp256k1::verification_only();
    let msg = Message::from_digest_slice(&hash).map_err(|_| "Bad hash")?;
    let sig = Signature::from_compact(&sig_bytes).map_err(|_| "Bad sig")?;
    
    Ok(secp.verify_ecdsa(&msg, &sig, &pubkey.0).is_ok())
}