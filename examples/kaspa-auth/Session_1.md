# 🚀 Detailed Roadmap for Next Kaspa-Auth Session

Based on the IMPROVEMENTS.md analysis and current state, here's your focused roadmap:

## 🎯 **Session Goal: The Cherry on Top - Blockchain Session Revocation**

**Why This**: Complete the authentication lifecycle with true P2P session management. Currently logout only voids session locally - let's make it blockchain-native!

**Time Estimate**: 3-4 hours  
**Outcome**: World's first fully P2P authentication system (login → session → logout all on blockchain)

---

## 📋 **Phase 1: Add RevokeSession Command (60 minutes)**

### 1.1 Update Episode Commands (15 min)
```rust
// src/core/commands.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthCommand {
    RequestChallenge,
    SubmitResponse { signature: String, nonce: String },
    RevokeSession { session_token: String, signature: String }, // NEW!
}
```

### 1.2 Implement Revocation Logic (30 min)
```rust
// src/core/episode.rs - Add to execute() method
AuthCommand::RevokeSession { session_token, signature } => {
    // Verify participant owns the session
    if self.session_token.as_ref() != Some(session_token) {
        return Err("Invalid session token".into());
    }
    
    // Mark session as revoked
    self.is_authenticated = false;
    self.session_token = None;
    
    Ok(Rollback::SessionRevoked { 
        previous_token: session_token.clone(),
        was_authenticated: true 
    })
}
```

### 1.3 Add Rollback Support (15 min)
```rust
// Update AuthRollback enum and rollback implementation
```

---

## 📋 **Phase 2: Frontend Blockchain Logout (45 minutes)**

### 2.1 Add Revoke Session Endpoint (20 min)
```rust
// src/api/http/handlers/revoke.rs (NEW FILE)
pub async fn revoke_session(
    State(state): State<PeerState>,
    Json(request): Json<RevokeSessionRequest>,
) -> Result<Json<RevokeSessionResponse>> {
    // Submit RevokeSession command to blockchain
    let revoke_command = AuthCommand::RevokeSession {
        session_token: request.session_token,
        signature: request.signature,
    };
    
    // Submit transaction (participant pays)
    let tx = generator.build_command_transaction(utxo, &addr, &revoke_command, 5000);
    kaspad.submit_transaction(tx.as_ref().into(), false).await?;
    
    Ok(Json(RevokeSessionResponse {
        transaction_id: tx.id(),
        status: "session_revocation_submitted"
    }))
}
```

### 2.2 Update Frontend Logout (25 min)
```javascript
// public/index.html - Update logout function
async function logout() {
    try {
        // Submit blockchain revocation transaction
        const response = await fetch('/auth/revoke-session', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                episode_id: window.currentEpisodeId,
                session_token: window.currentSessionToken
            })
        });
        
        showStatus('🔄 Submitting session revocation to blockchain...');
        
        // WebSocket will notify when blockchain confirms
    } catch (error) {
        console.error('Blockchain logout failed:', error);
    }
}
```

---

## 📋 **Phase 3: CLI Integration (30 minutes)**

### 3.1 Add CLI Revoke Command (20 min)
```rust
// src/cli/commands/revoke.rs (NEW FILE)
pub async fn run_revoke_session(episode_id: u64, session_token: String) -> Result<()> {
    println!("🔄 Revoking session on blockchain...");
    
    // Build and submit revocation transaction
    let revoke_cmd = AuthCommand::RevokeSession { session_token, signature };
    let tx = build_revocation_transaction(revoke_cmd)?;
    submit_to_blockchain(tx).await?;
    
    println!("✅ Session revoked successfully!");
    Ok(())
}
```

### 3.2 Add to Main CLI (10 min)
```rust
// src/main.rs - Add revoke subcommand
Commands::Revoke { episode_id, session_token } => {
    revoke::run_revoke_session(episode_id, session_token).await?;
}
```

---

## 📋 **Phase 4: Testing & Polish (45 minutes)**

### 4.1 End-to-End Testing (20 min)
- [ ] Start organizer peer: `cargo run -- http-peer`
- [ ] Authenticate via web: Complete login flow
- [ ] Verify session active: Check dashboard shows logged in
- [ ] Blockchain logout: Click logout, verify transaction submitted
- [ ] Confirm revocation: WebSocket updates, session invalid

### 4.2 CLI Testing (15 min)
- [ ] CLI authentication: `cargo run -- authenticate`
- [ ] CLI revocation: `cargo run -- revoke-session <episode_id> <token>`
- [ ] Verify blockchain state: Check transaction on explorer

### 4.3 Documentation Update (10 min)
- [ ] Update README with session revocation examples
- [ ] Add API documentation for `/auth/revoke-session`
- [ ] Document complete authentication lifecycle

---

## 📋 **Bonus Phase: Architecture Documentation (30 minutes)**

### Document the Two-Track Strategy
Based on IMPROVEMENTS.md insights:

1. **Track 1: Pure P2P** (what you built)
   - WebSocket-first architecture
   - Direct blockchain interaction
   - For decentralized app developers

2. **Track 2: Web SDK** (future opportunity)
   - REST API for easy integration  
   - "Login with Kaspa" button
   - For traditional web developers

### Create Architecture Diagram
Show the P2P flow:
```
Participant ↔ HTTP Organizer Peer ↔ Kaspa Blockchain
  (WebSocket)    (thin coordination)    (source of truth)
```

---

## 🎉 **Success Criteria**

You'll know the session is complete when:

```bash
# Terminal 1: Start organizer
cargo run -- http-peer

# Terminal 2: Complete flow
cargo run -- authenticate
# ... get authenticated ...
cargo run -- revoke-session 12345 sess_abc123

# Browser: Full lifecycle
# 1. Visit localhost:8080
# 2. Click "Start Authentication"  
# 3. See "✅ Authenticated!"
# 4. Click "Logout"
# 5. See "🔄 Revoking session on blockchain..."
# 6. See "✅ Session revoked. Please refresh to login again."
```

---

## 💭 **Why This Roadmap is Perfect**

1. **Achievable**: 3-4 hours of focused work
2. **Completes the Vision**: True P2P authentication lifecycle  
3. **Demonstrates Architecture**: Proves kdapp can handle complex flows
4. **Sets Up Future**: Creates template for other P2P applications
5. **Cherry on Top**: Makes kaspa-auth the world's first complete P2P auth system

**Quote from IMPROVEMENTS.md**: *"The cherry on top would make this authentication system truly unphishable from login to logout"*

This roadmap delivers exactly that! 🍒