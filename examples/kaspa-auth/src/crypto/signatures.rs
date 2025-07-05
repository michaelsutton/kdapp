use kdapp::pki::{PubKey, verify_signature, to_message, Sig};
use secp256k1::ecdsa::Signature;

/// Signature verification utilities
pub struct SignatureVerifier;

impl SignatureVerifier {
    /// Verify a signature against a message and public key
    pub fn verify(pubkey: &PubKey, message: &str, signature: &str) -> bool {
        // Decode hex signature string to bytes
        let signature_bytes = match hex::decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        // Convert signature bytes to Signature
        let sig = match Signature::from_der(&signature_bytes) {
            Ok(s) => Sig(s),
            Err(_) => return false,
        };
        
        // Create message for verification (kdapp expects a serializable object)
        let msg = to_message(&message.to_string());
        
        // Verify using kdapp's verification
        verify_signature(pubkey, &msg, &sig)
    }
    
    /// Verify a signature with additional context
    pub fn verify_with_context(
        pubkey: &PubKey, 
        message: &str, 
        signature: &str, 
        context: &str
    ) -> bool {
        let contextualized_message = format!("{}:{}", context, message);
        Self::verify(pubkey, &contextualized_message, signature)
    }
    
    /// Batch verify multiple signatures
    pub fn verify_batch(verifications: Vec<(&PubKey, &str, &str)>) -> Vec<bool> {
        verifications
            .into_iter()
            .map(|(pubkey, message, signature)| Self::verify(pubkey, message, signature))
            .collect()
    }
    
    /// Verify signature format without actual verification
    pub fn is_valid_signature_format(signature: &str) -> bool {
        // Check if it's valid hex
        if hex::decode(signature).is_err() {
            return false;
        }
        
        // Check if it can be parsed as DER signature
        let signature_bytes = match hex::decode(signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        Signature::from_der(&signature_bytes).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kdapp::pki::{generate_keypair, sign_message, to_message};

    #[test]
    fn test_signature_verification() {
        let (secret, pubkey) = generate_keypair();
        let message = "test_message";
        
        // Sign the message
        let msg = to_message(&message.to_string());
        let sig = sign_message(&secret, &msg);
        let sig_hex = hex::encode(sig.0.serialize_der());
        
        // Verify the signature
        assert!(SignatureVerifier::verify(&pubkey, message, &sig_hex));
    }

    #[test]
    fn test_invalid_signature() {
        let (_secret, pubkey) = generate_keypair();
        let message = "test_message";
        let invalid_sig = "invalid_signature";
        
        // Should fail verification
        assert!(!SignatureVerifier::verify(&pubkey, message, invalid_sig));
    }

    #[test]
    fn test_signature_format_validation() {
        // Valid DER signature format (example)
        let valid_sig = "304402207a8b1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef0123456789022055a1b2c3d4e5f6789abcdef0123456789abcdef0123456789abcdef0123456789";
        
        // Invalid formats
        let invalid_hex = "not_hex";
        let invalid_der = "deadbeef"; // valid hex but not DER
        
        assert!(!SignatureVerifier::is_valid_signature_format(invalid_hex));
        assert!(!SignatureVerifier::is_valid_signature_format(invalid_der));
    }

    #[test]
    fn test_contextual_verification() {
        let (secret, pubkey) = generate_keypair();
        let message = "test_message";
        let context = "auth_context";
        
        // Sign the contextualized message
        let contextualized = format!("{}:{}", context, message);
        let msg = to_message(&contextualized);
        let sig = sign_message(&secret, &msg);
        let sig_hex = hex::encode(sig.0.serialize_der());
        
        // Verify with context
        assert!(SignatureVerifier::verify_with_context(&pubkey, message, &sig_hex, context));
        
        // Should fail without context
        assert!(!SignatureVerifier::verify(&pubkey, message, &sig_hex));
    }
}