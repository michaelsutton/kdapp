# 🏗️ Session 2: Extract Pure P2P Example (Architecture Proof)

**Prerequisites**: Session 1 completed (blockchain session revocation working)

## 🎯 **Session Goal: Prove the Minimal Architecture**

**Why This**: Extract a clean WebSocket-first example that demonstrates kaspa-auth can be built in ~600 lines instead of 2000+. This proves the architecture is sound and creates a template for future kdapp projects.

**Time Estimate**: 4-5 hours  
**Outcome**: Working `examples/pure-p2p/` that shows the elegant core architecture

---

## 📋 **Phase 1: Create Pure P2P Structure (45 minutes)**

### 1.1 Set Up Directory Structure (15 min)
```bash
mkdir -p examples/pure-p2p/src
mkdir -p examples/pure-p2p/public
touch examples/pure-p2p/Cargo.toml
touch examples/pure-p2p/src/main.rs
touch examples/pure-p2p/src/websocket.rs
touch examples/pure-p2p/src/episode.rs
touch examples/pure-p2p/public/index.html
```

### 1.2 Create Minimal Cargo.toml (10 min)
```toml
[package]
name = "kaspa-auth-pure-p2p"
version = "0.1.0"
edition = "2021"

[dependencies]
kdapp = { path = "../../../kdapp" }
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.20"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
secp256k1 = { version = "0.29", features = ["global-context", "rand-std"] }
rand = "0.8"
```

### 1.3 Plan File Responsibilities (20 min)
- `main.rs` (50 lines): WebSocket server setup, kdapp engine integration
- `websocket.rs` (150 lines): WebSocket message handling, event relay
- `episode.rs` (200 lines): Copy core authentication episode
- `index.html` (200 lines): WebSocket-only frontend (no HTTP fallbacks)

---

## 📋 **Phase 2: Implement WebSocket-First Server (90 minutes)**

### 2.1 Create Minimal Main Server (30 min)
```rust
// src/main.rs
use tokio_tungstenite::{accept_async, tungstenite::Message};
use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Start kdapp engine
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let auth_handler = AuthHandler::new();
    
    tokio::spawn(async move {
        let mut engine = kdapp::Engine::new(rx);
        engine.start(vec![auth_handler]);
    });
    
    // 2. Start WebSocket server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("🚀 Pure P2P Auth Server running on ws://127.0.0.1:8080");
    
    let (broadcast_tx, _) = broadcast::channel(100);
    
    while let Ok((stream, _)) = listener.accept().await {
        let broadcast_tx = broadcast_tx.clone();
        let tx = tx.clone();
        
        tokio::spawn(handle_connection(stream, broadcast_tx, tx));
    }
    
    Ok(())
}
```

### 2.2 Implement WebSocket Handler (45 min)
```rust
// src/websocket.rs
pub async fn handle_connection(
    stream: TcpStream, 
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
    engine_tx: mpsc::Sender<EngineCommand>
) {
    let ws_stream = accept_async(stream).await.expect("WebSocket handshake");
    let (ws_tx, mut ws_rx) = ws_stream.split();
    
    // Listen for participant messages
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(participant_msg) = serde_json::from_str::<ParticipantMessage>(&text) {
                    handle_participant_message(participant_msg, &engine_tx).await;
                }
            }
            _ => break,
        }
    }
}

async fn handle_participant_message(
    msg: ParticipantMessage, 
    engine_tx: &mpsc::Sender<EngineCommand>
) {
    match msg {
        ParticipantMessage::RequestAuth { public_key } => {
            // Forward to kdapp engine
            let _ = engine_tx.send(EngineCommand::StartAuth { public_key }).await;
        }
        ParticipantMessage::SubmitSignature { episode_id, signature, nonce } => {
            let _ = engine_tx.send(EngineCommand::VerifyAuth { 
                episode_id, signature, nonce 
            }).await;
        }
    }
}
```

### 2.3 Add Blockchain Event Broadcasting (15 min)
```rust
// Add to websocket.rs
pub async fn broadcast_blockchain_events(
    mut blockchain_rx: mpsc::Receiver<BlockchainEvent>,
    broadcast_tx: broadcast::Sender<WebSocketMessage>
) {
    while let Some(event) = blockchain_rx.recv().await {
        let ws_msg = match event {
            BlockchainEvent::ChallengeIssued { episode_id, nonce } => {
                WebSocketMessage::ChallengeReady { episode_id, nonce }
            }
            BlockchainEvent::AuthComplete { episode_id, session_token } => {
                WebSocketMessage::AuthSuccess { episode_id, session_token }
            }
        };
        
        let _ = broadcast_tx.send(ws_msg);
    }
}
```

---

## 📋 **Phase 3: Copy and Adapt Episode Logic (60 minutes)**

### 3.1 Copy Core Episode (30 min)
```rust
// src/episode.rs - Copy from ../kaspa-auth/src/core/episode.rs
// Keep the same 200-line authentication logic
// Remove HTTP-specific dependencies
// Ensure it works with pure kdapp patterns
```

### 3.2 Adapt for WebSocket Communication (30 min)
```rust
// Update episode to emit events instead of HTTP responses
impl Episode for SimpleAuth {
    fn execute(&mut self, command: &AuthCommand, auth: Option<PubKey>, meta: &PayloadMetadata) -> Result<Rollback, Error> {
        match command {
            AuthCommand::RequestChallenge => {
                let nonce = generate_challenge();
                self.challenge = Some(nonce.clone());
                
                // Emit event for WebSocket broadcast
                self.emit_event(AuthEvent::ChallengeIssued { 
                    episode_id: meta.episode_id, 
                    nonce 
                });
                
                Ok(Rollback::Challenge(None))
            }
            // ... rest of the logic
        }
    }
}
```

---

## 📋 **Phase 4: Create WebSocket-Only Frontend (75 minutes)**

### 4.1 HTML Structure (25 min)
```html
<!-- public/index.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Kaspa Auth - Pure P2P</title>
    <style>/* Minimal styling */</style>
</head>
<body>
    <div id="app">
        <h1>🔗 Pure P2P Authentication</h1>
        <div id="status">Connecting to WebSocket...</div>
        
        <div id="auth-section" style="display: none;">
            <button id="start-auth">🚀 Start Authentication</button>
            <div id="challenge-section" style="display: none;">
                <p>Challenge: <code id="challenge-display"></code></p>
                <button id="sign-challenge">✍️ Sign Challenge</button>
            </div>
            <div id="success-section" style="display: none;">
                <p>✅ Authenticated!</p>
                <p>Session: <code id="session-display"></code></p>
                <button id="logout">🚪 Logout</button>
            </div>
        </div>
    </div>
    
    <script>/* WebSocket-only JavaScript */</script>
</body>
</html>
```

### 4.2 WebSocket-Only JavaScript (50 min)
```javascript
// No HTTP requests - only WebSocket communication
class PureP2PAuth {
    constructor() {
        this.ws = null;
        this.currentEpisode = null;
        this.sessionToken = null;
        this.connect();
    }
    
    connect() {
        this.ws = new WebSocket('ws://localhost:8080');
        
        this.ws.onopen = () => {
            document.getElementById('status').textContent = '🟢 Connected to P2P network';
            document.getElementById('auth-section').style.display = 'block';
        };
        
        this.ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            this.handleMessage(message);
        };
        
        this.ws.onclose = () => {
            document.getElementById('status').textContent = '🔴 Disconnected from P2P network';
            setTimeout(() => this.connect(), 3000);
        };
    }
    
    handleMessage(message) {
        switch(message.type) {
            case 'challenge_ready':
                this.showChallenge(message.nonce);
                break;
            case 'auth_success':
                this.showSuccess(message.session_token);
                break;
            case 'auth_failed':
                this.showError(message.reason);
                break;
        }
    }
    
    startAuth() {
        const publicKey = localStorage.getItem('kaspa_public_key') || this.generateKeypair();
        
        this.ws.send(JSON.stringify({
            type: 'request_auth',
            public_key: publicKey
        }));
        
        document.getElementById('status').textContent = '🔄 Requesting authentication challenge...';
    }
    
    signChallenge(nonce) {
        // Sign with local keypair (simplified for demo)
        const signature = this.signMessage(nonce);
        
        this.ws.send(JSON.stringify({
            type: 'submit_signature',
            episode_id: this.currentEpisode,
            signature: signature,
            nonce: nonce
        }));
        
        document.getElementById('status').textContent = '🔄 Submitting signed challenge to blockchain...';
    }
    
    // No HTTP fallbacks - pure WebSocket communication!
}

const auth = new PureP2PAuth();
```

---

## 📋 **Phase 5: Testing and Documentation (30 minutes)**

### 5.1 End-to-End Testing (20 min)
- [ ] Start pure P2P server: `cd examples/pure-p2p && cargo run`
- [ ] Open browser: `http://localhost:8080`
- [ ] Complete authentication flow via WebSocket only
- [ ] Verify no HTTP requests in browser dev tools
- [ ] Test real-time updates work

### 5.2 Document the Difference (10 min)
```markdown
# Pure P2P vs Web SDK

## Pure P2P (This Example)
- WebSocket-only communication
- Direct kdapp engine integration
- Real-time blockchain events
- ~600 lines total
- For P2P applications

## Web SDK (../kaspa-auth/)
- REST API coordination
- HTTP convenience layer
- Polling-based updates
- ~2000 lines total
- For traditional web apps
```

---

## 🎉 **Success Criteria**

You'll know Session 2 is complete when:

1. **Pure P2P server runs**: `cargo run` in `examples/pure-p2p/`
2. **WebSocket-only authentication**: Complete flow without any HTTP requests
3. **Real-time updates**: Instant blockchain event notifications
4. **Code comparison**: ~600 lines vs 2000+ in main project
5. **Clear separation**: No HTTP dependencies in pure P2P example

---

## 💭 **Why This Session is Critical**

1. **Proves Architecture**: Shows kaspa-auth CAN be simple
2. **Creates Template**: Other kdapp projects can copy this pattern
3. **Market Positioning**: Clear difference between P2P and web SDK approaches
4. **Developer Education**: Shows progression from complex to elegant

**Quote**: *"The pure P2P example will be the 'Hello World' for kdapp authentication"*

This session transforms the project from "over-engineered authentication" to "strategic architecture showcase"! 🎯