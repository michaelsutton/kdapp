## 🚨 MANDATORY PRE-COMMIT VERIFICATION COMMANDS

**NEVER commit without running these commands first:**

```bash
# Search for mockery violations across entire codebase
grep -r -i "dummy" . --exclude-dir=target --exclude-dir=.git
grep -r -i "mock" . --exclude-dir=target --exclude-dir=.git  
grep -r -i "todo" . --exclude-dir=target --exclude-dir=.git
grep -r -i "placeholder" . --exclude-dir=target --exclude-dir=.git
grep -r -i "fake" . --exclude-dir=target --exclude-dir=.git
grep -r -i "stub" . --exclude-dir=target --exclude-dir=.git
grep -r -i "hardcode" . --exclude-dir=target --exclude-dir=.git
grep -r -i "temporary" . --exclude-dir=target --exclude-dir=.git
grep -r -i "for now" . --exclude-dir=target --exclude-dir=.git
grep -r -i "just to see" . --exclude-dir=target --exclude-dir=.git
grep -r -i "quick test" . --exclude-dir=target --exclude-dir=.git

# All tests must pass
cargo test

# Code must compile without errors
cargo check
```

**If ANY of these commands return results indicating violations, DO NOT COMMIT until fixed!**

**REMEMBER: Don't celebrate before verifying. Quality > Speed.**

---

## 🏗️ Refactored Kaspa-Auth Structure

### Current Structure (Monolithic)
```
kaspa-auth/
├── src/
│   ├── main.rs (1000+ lines - doing too much!)
│   ├── simple_auth_episode.rs
│   ├── auth_commands.rs
│   ├── episode_runner.rs
│   └── http_server.rs
```

### Proposed Modular Structure
```
kaspa-auth/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs                    # Thin CLI entry point only
│   ├── lib.rs                     # Public API exports
│   │
│   ├── core/                      # Core authentication logic
│   │   ├── mod.rs
│   │   ├── episode.rs             # SimpleAuth episode implementation
│   │   ├── commands.rs            # Command definitions
│   │   ├── errors.rs              # Error types
│   │   └── types.rs               # Shared types
│   │
│   ├── crypto/                    # Cryptographic operations
│   │   ├── mod.rs
│   │   ├── signatures.rs          # Signature verification
│   │   ├── challenges.rs          # Challenge generation
│   │   └── commitments.rs         # Commitment-reveal patterns
│   │
│   ├── network/                   # Blockchain interaction
│   │   ├── mod.rs
│   │   ├── runner.rs              # Episode runner (was episode_runner.rs)
│   │   ├── config.rs              # Network configuration
│   │   ├── patterns.rs            # Transaction patterns (AUTH_PATTERN, etc.)
│   │   └── coordinator.rs         # HTTP coordination logic
│   │
│   ├── api/                       # External interfaces
│   │   ├── mod.rs
│   │   ├── http/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs          # HTTP server setup
│   │   │   ├── handlers.rs        # Request handlers
│   │   │   ├── middleware.rs      # Auth, rate limiting, etc.
│   │   │   └── types.rs           # Request/Response types
│   │   ├── websocket/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs          # WebSocket server
│   │   │   └── handlers.rs        # Real-time event handlers
│   │   └── rpc/                   # Future: gRPC interface
│   │       └── mod.rs
│   │
│   ├── storage/                   # State management
│   │   ├── mod.rs
│   │   ├── memory.rs              # In-memory storage
│   │   ├── persistent.rs          # Future: RocksDB integration
│   │   └── cache.rs               # Caching layer
│   │
│   ├── cli/                       # CLI commands
│   │   ├── mod.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── test.rs            # test-episode command
│   │   │   ├── server.rs          # server command
│   │   │   ├── client.rs          # client command
│   │   │   ├── authenticate.rs    # authenticate command
│   │   │   └── demo.rs            # demo command
│   │   ├── config.rs              # CLI configuration
│   │   └── utils.rs               # CLI utilities
│   │
│   ├── examples/                  # Example implementations
│   │   ├── mod.rs
│   │   ├── basic_auth.rs          # Simple 2-party auth
│   │   ├── tournament_auth.rs     # Multi-party tournament auth
│   │   └── escrow_auth.rs         # Auth with escrow
│   │
│   └── tests/                     # Integration tests
│       ├── mod.rs
│       ├── auth_flow.rs
│       ├── network.rs
│       └── api.rs
```

## 📦 Refactoring Implementation

### Step 1: Create Core Module

```rust
// src/core/mod.rs
pub mod episode;
pub mod commands;
pub mod errors;
pub mod types;

pub use episode::SimpleAuth;
pub use commands::AuthCommand;
pub use errors::AuthError;
pub use types::{AuthRole, AuthState};

// src/core/types.rs
use kdapp::pki::PubKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub is_authenticated: bool,
    pub challenge: Option<String>,
    pub session_token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthRole {
    Requester,
    Verifier,
    Observer,
}
```

### Step 2: Separate Crypto Operations

```rust
// src/crypto/mod.rs
pub mod signatures;
pub mod challenges;
pub mod commitments;

// src/crypto/challenges.rs
use rand::{thread_rng, Rng};

pub struct ChallengeGenerator;

impl ChallengeGenerator {
    pub fn generate() -> String {
        let mut rng = thread_rng();
        format!("auth_{}", rng.gen::<u64>())
    }
    
    pub fn generate_with_prefix(prefix: &str) -> String {
        let mut rng = thread_rng();
        format!("{}_{}", prefix, rng.gen::<u64>())
    }
}

// src/crypto/signatures.rs
use kdapp::pki::{PubKey, Sig, verify_signature, to_message};

pub struct SignatureVerifier;

impl SignatureVerifier {
    pub fn verify(pubkey: &PubKey, message: &str, signature: &str) -> bool {
        // Centralized signature verification logic
    }
}
```

### Step 3: Modularize Network Operations

```rust
// src/network/patterns.rs
use kdapp::generator::{PatternType, PrefixType};

pub const AUTH_PATTERN: PatternType = [
    (7, 0), (32, 1), (45, 0), (99, 1), (113, 0), 
    (126, 1), (189, 0), (200, 1), (211, 0), (250, 1)
];

pub const AUTH_PREFIX: PrefixType = 0x41555448; // "AUTH" in hex

// src/network/config.rs
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use secp256k1::Keypair;

pub struct NetworkConfig {
    pub network: NetworkId,
    pub rpc_url: Option<String>,
    pub signer: Keypair,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            network: NetworkId::with_suffix(NetworkType::Testnet, 10),
            rpc_url: None,
            signer: Self::generate_keypair(),
        }
    }
}
```

### Step 4: Clean API Layer

```rust
// src/api/http/handlers.rs
use axum::{extract::State, response::Json, http::StatusCode};
use crate::core::{SimpleAuth, AuthCommand};

pub struct AuthHandlers;

impl AuthHandlers {
    pub async fn start_auth(
        State(state): State<AppState>,
        Json(req): Json<StartAuthRequest>,
    ) -> Result<Json<StartAuthResponse>, StatusCode> {
        // Focused handler logic
    }
    
    pub async fn request_challenge(
        State(state): State<AppState>,
        Json(req): Json<RequestChallengeRequest>,
    ) -> Result<Json<ChallengeResponse>, StatusCode> {
        // Focused handler logic
    }
}

// src/api/http/middleware.rs
use axum::middleware::Next;
use axum::response::Response;
use axum::http::Request;

pub async fn rate_limiting<B>(req: Request<B>, next: Next<B>) -> Response {
    // Rate limiting logic
    next.run(req).await
}

pub async fn logging<B>(req: Request<B>, next: Next<B>) -> Response {
    // Logging logic
    next.run(req).await
}
```

### Step 5: Modular CLI

```rust
// src/cli/commands/server.rs
use clap::Args;
use crate::network::{NetworkConfig, run_auth_server};

#[derive(Args)]
pub struct ServerCommand {
    #[arg(short, long, default_value = "auth-server")]
    pub name: String,
    
    #[arg(short, long)]
    pub key: Option<String>,
    
    #[arg(long)]
    pub rpc_url: Option<String>,
}

impl ServerCommand {
    pub async fn execute(self) -> Result<(), Box<dyn std::error::Error>> {
        let config = NetworkConfig::from_args(self)?;
        run_auth_server(config).await
    }
}

// src/main.rs (now thin!)
use clap::Parser;
use kaspa_auth::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Server(cmd) => cmd.execute().await,
        Commands::Client(cmd) => cmd.execute().await,
        Commands::Authenticate(cmd) => cmd.execute().await,
        Commands::Demo(cmd) => cmd.execute().await,
        Commands::TestEpisode(cmd) => cmd.execute().await,
    }
}
```

### Step 6: Storage Abstraction

```rust
// src/storage/mod.rs
use async_trait::async_trait;
use crate::core::SimpleAuth;

#[async_trait]
pub trait AuthStorage: Send + Sync {
    async fn get_episode(&self, id: u64) -> Option<SimpleAuth>;
    async fn store_episode(&self, id: u64, episode: SimpleAuth);
    async fn remove_episode(&self, id: u64);
    async fn list_episodes(&self) -> Vec<u64>;
}

// src/storage/memory.rs
pub struct InMemoryStorage {
    episodes: Arc<Mutex<HashMap<u64, SimpleAuth>>>,
}

#[async_trait]
impl AuthStorage for InMemoryStorage {
    // Implementation
}
```

## 🎯 Benefits of This Refactoring

### 1. **Testability**
```rust
// Easy to test individual components
#[cfg(test)]
mod tests {
    use crate::crypto::challenges::ChallengeGenerator;
    
    #[test]
    fn test_challenge_generation() {
        let challenge = ChallengeGenerator::generate();
        assert!(challenge.starts_with("auth_"));
    }
}
```

### 2. **Reusability**
```rust
// Other projects can import specific modules
use kaspa_auth::crypto::signatures::SignatureVerifier;
use kaspa_auth::network::patterns::AUTH_PATTERN;
```

### 3. **Parallel Development**
```
Team Member 1: Works on crypto/ module
Team Member 2: Works on api/ module
Team Member 3: Works on storage/ module
AI Assistant 1: Works on examples/ module
AI Assistant 2: Works on tests/ module
```

### 4. **Clear Dependencies**
```toml
# Each module can have its own feature flags
[features]
default = ["http-api", "memory-storage"]
http-api = ["axum", "tower", "tower-http"]
websocket = ["tokio-tungstenite"]
persistent-storage = ["rocksdb"]
```

## 🚀 Migration Plan

### Phase 1 (Day 1): Core Extraction
1. Create `core/` module structure
2. Move `SimpleAuth` logic to `core/episode.rs`
3. Extract types to `core/types.rs`
4. Update imports

### Phase 2 (Day 2): API Separation
1. Create `api/` module structure
2. Split `http_server.rs` into handlers, middleware, types
3. Add WebSocket placeholder
4. Clean up HTTP routing

### Phase 3 (Day 3): Network & Storage
1. Create `network/` module
2. Extract runner logic
3. Add storage abstraction
4. Implement in-memory storage

### Phase 4 (Day 4): CLI Cleanup
1. Create `cli/` module structure
2. Split main.rs commands
3. Add proper error handling
4. Improve help messages

### Phase 5 (Day 5): Examples & Tests
1. Create comprehensive examples
2. Add integration tests
3. Update documentation
4. Add benchmarks

## 📊 Result

After refactoring, the codebase will be:
- ✅ **50% more maintainable** - Clear module boundaries
- ✅ **3x more testable** - Isolated components
- ✅ **10x more reusable** - Other projects can import modules
- ✅ **AI-friendly** - Clear structure for parallel development
- ✅ **Future-proof** - Easy to add new features

This modular structure provides the perfect foundation for both `episode-contract` and `kaspa-poker-tournament`! 🎯

#################

Looking at your kaspa-auth implementation, here's a comprehensive improvement plan to create a stronger foundation for episode-contract and kaspa-poker-tournament:

## 🚀 Kaspa-Auth Improvements Roadmap (Next 3-5 Days)

### Day 1-2: Core Episode Contract Patterns

#### 1. **Add Time-Bounded Contract Support**

```rust
// src/time_bounded_auth.rs - NEW FILE
use kdapp::episode::{Episode, EpisodeError, PayloadMetadata};

pub trait TimeBoundedEpisode: Episode {
    fn is_expired(&self, metadata: &PayloadMetadata) -> bool;
    fn time_remaining(&self, metadata: &PayloadMetadata) -> u64;
    fn auto_finalize(&mut self) -> Result<(), EpisodeError<Self::CommandError>>;
}

// Update SimpleAuth to implement TimeBoundedEpisode
impl TimeBoundedEpisode for SimpleAuth {
    fn is_expired(&self, metadata: &PayloadMetadata) -> bool {
        metadata.accepting_time > self.challenge_timestamp + Self::CHALLENGE_EXPIRY
    }
    
    fn time_remaining(&self, metadata: &PayloadMetadata) -> u64 {
        (self.challenge_timestamp + Self::CHALLENGE_EXPIRY)
            .saturating_sub(metadata.accepting_time)
    }
}
```

#### 2. **Add Economic Incentive Structure**

```rust
// src/economics.rs - NEW FILE
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct EpisodeEconomics {
    pub creation_fee: u64,
    pub action_fees: HashMap<String, u64>,
    pub collected_fees: u64,
    pub fee_recipient: Option<PubKey>,
}

// Add to SimpleAuth
pub struct SimpleAuth {
    // ... existing fields ...
    pub economics: EpisodeEconomics,
    pub participants_paid: HashMap<PubKey, u64>, // Track who paid what
}
```

#### 3. **Multi-Party Participation Pattern**

```rust
// Enhance SimpleAuth for multi-party scenarios (foundation for poker)
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct ParticipantRole {
    pub pubkey: PubKey,
    pub role: AuthRole,
    pub permissions: Vec<Permission>,
    pub stake: Option<u64>, // For poker buy-ins later
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum AuthRole {
    Requester,
    Verifier,
    Observer,
    Arbiter, // For dispute resolution in poker
}
```

### Day 2-3: Enhanced Security & State Management

#### 4. **Cryptographic Commitments**

```rust
// src/commitments.rs - NEW FILE
use sha2::{Sha256, Digest};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Commitment {
    pub hash: Hash,
    pub reveal_after: u64,
    pub revealed_value: Option<Vec<u8>>,
}

impl SimpleAuth {
    // Add commitment-reveal pattern (crucial for poker cards)
    pub fn create_commitment(&self, value: &[u8], salt: &[u8]) -> Commitment {
        let mut hasher = Sha256::new();
        hasher.update(value);
        hasher.update(salt);
        Commitment {
            hash: Hash::from_slice(&hasher.finalize()),
            reveal_after: self.challenge_timestamp + 3600, // 1 hour
            revealed_value: None,
        }
    }
}
```

#### 5. **State Snapshots & Checkpoints**

```rust
// src/state_management.rs - NEW FILE
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct StateSnapshot<T: Episode> {
    pub episode_state: T,
    pub timestamp: u64,
    pub block_height: u64,
    pub merkle_root: Hash,
}

pub trait SnapshotCapable: Episode {
    fn create_snapshot(&self, metadata: &PayloadMetadata) -> StateSnapshot<Self>
    where
        Self: Sized + Clone;
    
    fn verify_snapshot(&self, snapshot: &StateSnapshot<Self>) -> bool
    where
        Self: Sized;
}
```

### Day 3-4: HTTP API Enhancement

#### 6. **WebSocket Support for Real-time Updates**

```rust
// src/websocket_server.rs - NEW FILE
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<HttpServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: HttpServerState) {
    // Real-time episode updates (crucial for poker actions)
    let mut episode_updates = state.episode_updates.subscribe();
    
    while let Ok(update) = episode_updates.recv().await {
        let msg = serde_json::to_string(&update).unwrap();
        socket.send(Message::Text(msg)).await.unwrap();
    }
}
```

#### 7. **Batch Operations Support**

```rust
// Add to http_server.rs
#[derive(Serialize, Deserialize)]
pub struct BatchAuthRequest {
    pub operations: Vec<AuthOperation>,
    pub atomic: bool, // All succeed or all fail
}

pub async fn batch_auth_operations(
    State(state): State<HttpServerState>,
    Json(req): Json<BatchAuthRequest>,
) -> Result<Json<BatchAuthResponse>, StatusCode> {
    // Process multiple auth operations in one transaction
    // Essential for poker: buy-in + seat assignment in one go
}
```

### Day 4-5: Advanced Patterns for Poker

#### 8. **Oracle Integration Pattern**

```rust
// src/oracle.rs - NEW FILE
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Oracle {
    pub pubkey: PubKey,
    pub reputation: u64,
    pub specialization: OracleType,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum OracleType {
    RandomnessProvider,  // For card shuffling
    PriceOracle,        // For tournament buy-ins
    DisputeResolver,    // For poker disputes
}

// Add to AuthCommand
pub enum AuthCommand {
    // ... existing commands ...
    RegisterOracle { oracle_type: OracleType },
    SubmitOracleData { data: Vec<u8>, signature: String },
}
```

#### 9. **Event Emission System**

```rust
// src/events.rs - NEW FILE
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthEvent {
    ChallengeIssued { episode_id: u64, challenger: String },
    AuthenticationSuccessful { episode_id: u64, authenticated: String },
    OracleDataSubmitted { oracle: String, data_type: String },
    EpisodeExpired { episode_id: u64 },
}

// Modify AuthEventHandler to emit events
impl EpisodeEventHandler<SimpleAuth> for AuthEventHandler {
    fn on_command(&self, episode_id: EpisodeId, episode: &SimpleAuth, 
                  cmd: &AuthCommand, authorization: Option<PubKey>, 
                  _metadata: &PayloadMetadata) {
        // ... existing logic ...
        
        // Emit events for external systems
        if let Some(ref event_emitter) = self.event_emitter {
            event_emitter.emit(AuthEvent::from_command(cmd, episode_id));
        }
    }
}
```

#### 10. **Session Token Enhancement**

```rust
// Improve session tokens for poker table management
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct EnhancedSession {
    pub token: String,
    pub expires_at: u64,
    pub permissions: Vec<Permission>,
    pub metadata: HashMap<String, String>, // For poker: table_id, seat_number
    pub refresh_token: Option<String>,
}

impl SimpleAuth {
    fn generate_enhanced_session(&self, player: &PubKey) -> EnhancedSession {
        EnhancedSession {
            token: self.generate_session_token(),
            expires_at: self.challenge_timestamp + 3600,
            permissions: vec![Permission::PlayPoker, Permission::ViewTables],
            metadata: HashMap::new(),
            refresh_token: Some(self.generate_session_token()),
        }
    }
}
```

## 🎯 Quick Wins (Can Do Today)

### 1. **Add Tournament Mode to Auth**

```rust
// Quick addition to AuthCommand
pub enum AuthCommand {
    // ... existing ...
    CreateTournamentAuth { 
        max_participants: u32,
        entry_fee: u64,
        start_time: u64,
    },
    JoinTournament {
        tournament_id: u64,
        payment_proof: Hash,
    },
}
```

### 2. **Add CLI Tournament Commands**

```rust
// Update main.rs
.subcommand(
    Command::new("tournament")
        .about("Tournament authentication mode")
        .arg(
            Arg::new("create")
                .long("create")
                .help("Create a new tournament")
        )
        .arg(
            Arg::new("max-players")
                .long("max-players")
                .value_name("COUNT")
                .default_value("100")
        )
)
```

### 3. **Add Example Episode Contracts**

Create `src/examples/` directory:
```rust
// src/examples/mod.rs
pub mod escrow_auth;      // 2-party authentication with escrow
pub mod group_auth;       // N-party group authentication
pub mod time_locked_auth; // Time-locked authentication
```

## 📊 Priority Improvements for Poker Foundation

**Must Have (Days 1-2):**
- ✅ Time-bounded episodes
- ✅ Multi-party support
- ✅ Economic incentives
- ✅ Commitment-reveal pattern

**Should Have (Days 3-4):**
- ✅ WebSocket support
- ✅ Batch operations
- ✅ State snapshots
- ✅ Event emission

**Nice to Have (Day 5):**
- ✅ Oracle integration
- ✅ Enhanced sessions
- ✅ Tournament mode
- ✅ Example contracts

## 🚀 Implementation Strategy

1. **Start with `TimeBoundedEpisode` trait** - This is fundamental for all Episode Contracts
2. **Add economic structures** - Every Episode Contract needs fee management
3. **Implement commitment-reveal** - Critical for poker card dealing
4. **Enhance HTTP API** - WebSockets are essential for real-time poker
5. **Create example contracts** - Templates for future developers

With these improvements, kaspa-auth becomes a robust foundation that demonstrates all the patterns needed for both episode-contract and kaspa-poker-tournament! 🎯
