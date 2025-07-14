# Kaspa Auth - Episode-First Implementation

## 🌐 FUNDAMENTAL: kdapp is Peer-to-Peer, NOT Client-Server

### ❌ WRONG Hierarchical Thinking:
- "Server" controls authentication
- "Client" requests permission from server
- HTTP endpoints are the source of truth
- Traditional client-server architecture

### ✅ CORRECT Peer-to-Peer Reality:
- **HTTP Organizer Peer**: Organizes episode coordination via HTTP interface
- **Web Participant Peer**: Participant accessing via browser
- **CLI Participant Peer**: Participant accessing via command line
- **Blockchain**: The ONLY source of truth
- **Episodes**: Shared state between equal peers

### 🗣️ REQUIRED Terminology:
- **"HTTP Organizer Peer"** (not "server")
- **"Web Participant Peer"** (not "client")
- **"Organizer Peer"** (role, not hierarchy)
- **"Participant Peer"** (role, not hierarchy)
- **"Peer Address"** (not "server address" or "client address")

**Why This Matters**: When we use "server/client" language, we unconsciously default to hierarchical thinking patterns that are fundamentally wrong for kdapp architecture. This causes implementation bugs, security issues, and architectural confusion.

## 🎯 Goal
Build authentication as a Kaspa Episode FIRST, integrate wallet management SECOND.

## 🚫 What We're NOT Doing (Yet)
- Complex wallet management
- Multi-device flows  
- Recovery mechanisms
- Browser extensions
- React/WASM bindings
- Database persistence
- Production error handling

## ✅ What We ARE Doing
Simple challenge-response auth that works on Kaspa. Period.

---

## Phase 1: Minimal Viable Episode (Target: Working Demo in 3 days)

### Day 1: Core Episode Structure

**File: `src/simple_auth_episode.rs`**
```rust
// TODO: Create the simplest possible auth episode
// - Owner public key
// - Challenge string
// - Is authenticated boolean
// - No complex state, no device management, just auth
```

**File: `src/auth_commands.rs`**
```rust
// TODO: Just two commands
// - RequestChallenge
// - SubmitResponse { signature: String, nonce: String }
```

**File: `src/main.rs`**
```rust
// TODO: Basic CLI to test episode locally (no Kaspa yet)
// cargo run -- test-episode
```

### Day 2: Kaspa Integration

**Add to: `src/main.rs`**
```rust
// TODO: Connect to testnet-10
// - Send RequestChallenge as Kaspa transaction
// - Listen for challenge response
// - Send SubmitResponse as Kaspa transaction
// - Verify authentication succeeded
```

**File: `src/episode_runner.rs`**
```rust
// TODO: Minimal episode runner
// - Use kdapp's engine
// - Connect to Kaspa node
// - Process auth commands
```

### Day 3: Two-Party Demo

**File: `examples/auth_demo.rs`**
```rust
// TODO: Simple two-peer demo
// Terminal 1: cargo run --example auth_demo -- organizer-peer
// Terminal 2: cargo run --example auth_demo -- participant-peer --auth
```

**Success Criteria:**
- [ ] Alice initiates auth episode on Kaspa
- [ ] Bob (organizer peer) sees request and sends challenge
- [ ] Alice signs challenge and responds
- [ ] Bob verifies and confirms authentication
- [ ] Both parties see "✅ Authenticated!" 

---

## Phase 2: Make It Useful (Days 4-7)

### Add Session Token
```rust
// TODO: After successful auth, generate session token
// - Add to AuthState: session_token: Option<String>
// - Return token to authenticated user
// - Basic expiry (hardcoded 1 hour)
```

### Add Basic API
```rust
// TODO: Minimal HTTP endpoints
// POST /auth/start -> returns episode_id
// GET /auth/challenge/{episode_id} -> returns nonce
// POST /auth/verify -> returns session token
```

### Add Rate Limiting
```rust
// TODO: In-memory rate limit
// - Max 5 auth attempts per pubkey per hour
// - Simple HashMap counter
```

---

## Phase 3: Integration Decision Point (Day 8)

### Option A: Integrate Existing Wallet ✅
```rust
// If Phase 1 & 2 work perfectly:
use existing_project::wallet_guard::{WalletGuard, UnlockedWallet};

impl AuthWithWallet {
    pub async fn auth_with_existing_wallet(wallet: UnlockedWallet) -> Result<SessionToken> {
        // Reuse ALL your existing code
        let signature = wallet.sign_challenge(&challenge)?;
        // Just plug it into our simple auth episode
    }
}
```

### Option B: Minimal Auth-Only Wallet
```rust
// If wallet integration has issues:
struct MinimalAuthWallet {
    keypair: Keypair,  // Just for auth, no storage
}
```

### Option C: Hybrid Approach
```rust
// Support both:
enum AuthMethod {
    ExistingWallet(UnlockedWallet),
    SimpleKeypair(Keypair),
}
```

---

## 📁 File Structure (Keep It Simple!)

```
kaspa-auth/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # 20 lines
│   ├── simple_auth_episode.rs    # 100 lines
│   ├── auth_commands.rs          # 30 lines  
│   ├── episode_runner.rs         # 80 lines
│   └── main.rs                   # 100 lines
├── examples/
│   └── auth_demo.rs              # 150 lines
└── tests/
    └── basic_auth_test.rs        # 50 lines

Total: < 500 lines of code!
```

---

## 🧪 Test Commands (Progressive Complexity)

```bash
# Day 1: Test episode logic (no Kaspa)
cargo test test_auth_episode_logic

# Day 2: Test with local Kaspa node
cargo run -- test-local

# Day 3: Full demo on testnet-10
cargo run --example auth_demo -- organizer-peer
# In another terminal:
cargo run --example auth_demo -- participant-peer --key <your-test-key>

# Week 2: With API
curl -X POST http://localhost:8080/auth/start
```

---

## 🎯 Success Metrics

### Phase 1 Success = 
- [ ] Two peers can authenticate via Kaspa transactions
- [ ] Total code < 500 lines
- [ ] No external dependencies beyond kdapp + kaspa crates
- [ ] Works on testnet-10
- [ ] Zero wallet management code

### Phase 2 Success =
- [ ] Session tokens work
- [ ] Basic HTTP API works
- [ ] Still < 1000 lines total

### Phase 3 Success =
- [ ] Clean integration with existing wallet OR
- [ ] Working minimal wallet OR  
- [ ] Both options available

---

## 🚫 Common Pitfalls to Avoid

1. **DON'T** start with perfect error handling
2. **DON'T** build UI before CLI works
3. **DON'T** add features before basic auth works
4. **DON'T** optimize before it runs
5. **DON'T** integrate wallet until episode is proven

---

## 💬 Vibe-Coding Prompts

### Week 1 - Episode Focus
```
"Create a simple auth episode for Kaspa that does challenge-response authentication. 
Just two commands: RequestChallenge and SubmitResponse. Keep it under 200 lines."
```

### Week 2 - Integration Focus  
```
"Add a minimal HTTP API to the auth episode. Just three endpoints to start auth, 
get challenge, and verify response. No database, just in-memory."
```

### Week 3 - Wallet Integration
```
"I have an existing wallet_guard.rs file. Integrate it with the auth episode 
so users can sign challenges with their existing wallet."
```

---

## 🎉 Definition of Done

You know Phase 1 is complete when you can:

1. Open two terminals
2. Run organizer peer in terminal 1
3. Run participant peer in terminal 2  
4. See this interaction:

```
Terminal 1:
$ cargo run --example auth_demo -- organizer-peer
🎯 Auth organizer peer started on testnet-10
📨 Received auth request from kaspatest:xyz...
🎲 Sending challenge: "auth_1234567890"
✅ Signature verified! User authenticated.

Terminal 2:
$ cargo run --example auth_demo -- participant-peer --auth
🔑 Starting auth for key: kaspatest:xyz...
📨 Received challenge: "auth_1234567890"
✍️  Signing challenge...
✅ Authenticated! Session: sess_abc123
```

**That's it. Everything else comes after this works.**

---

*Remember: The kdapp philosophy is "fastest possible route". This roadmap is that route.*


## 🎯 The Correct Structure

**Add `kaspa-auth` to the examples folder!** Here's why:

### ✅ Proper Repository Structure:
```
kdapp/                          # Original repo (don't touch core!)
├── kdapp/                      # Core framework (don't modify!)
│   ├── src/
│   │   ├── engine.rs          # Core engine
│   │   ├── episode.rs         # Episode trait
│   │   └── ...
├── examples/                   # Your auth goes HERE!
│   ├── tictactoe/             # Existing example
│   └── kaspa-auth/            # NEW - Your auth implementation
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   └── simple_auth_episode.rs
│       └── README.md
└── Cargo.toml                 # Workspace root
```

### 📝 Update the workspace `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "kdapp", 
    "examples/tictactoe",
    "examples/kaspa-auth"    # Add this line!
]
```

### 🚀 Benefits of Examples Folder:

1. **Preserves Original Code**: Never modify the core framework
2. **Easy Updates**: Can pull upstream changes without conflicts
3. **Clear Separation**: Framework vs. implementation
4. **Follows Convention**: Just like tictactoe example
5. **Perfect for PRs**: Could contribute back as an example!

### 📁 Create the structure:
```bash
# From the root of your kdapp fork
cd examples
mkdir kaspa-auth
cd kaspa-auth

# Create Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "kaspa-auth"
version = "0.1.0"
edition = "2021"

[dependencies]
kdapp = { path = "../../kdapp" }
# ... other deps
EOF

# Create source directory
mkdir src
touch src/main.rs
touch src/simple_auth_episode.rs
```

### ❌ Why NOT to modify kdapp core:

1. **Merge Conflicts**: Hard to sync with upstream
2. **Breaks Separation**: Mixes framework with implementation  
3. **Harder to Debug**: Can't tell what's framework vs. your code
4. **Less Reusable**: Others can't use your auth as example

### 💡 Think of it like:
- `kdapp/` = The game engine (Unity/Unreal)
- `examples/kaspa-auth/` = Your game built on the engine

You wouldn't modify Unity's source to build your game, right? Same principle! 😊

So put your implementation in `examples/kaspa-auth/` and keep the kdapp core pristine! 🎯

---

## 🚨 CRITICAL ANTI-SHORTCUT ENGINEERING ALERT

### The "Mockery Moment" Detection System

When you find yourself thinking ANY of these thoughts:

❌ "Let's just mock the blockchain state for now..."
❌ "We'll simulate the episode coordination temporarily..."  
❌ "HTTP endpoints can return fake data until we figure out the real flow..."
❌ "Let's hardcode this session token logic..."
❌ "We'll build a simple version first, then add kdapp later..."
❌ "Let's use a fallback challenge for testing..." ← **PRODUCTION BUG EXAMPLE!**

### 🛑 IMMEDIATE ACTION REQUIRED:

**STOP CODING** and follow this exact process:

1. **Re-read Michael's kdapp README**: https://github.com/michaelsutton/kdapp
2. **Re-examine the tictactoe example**: See how it uses REAL transactions
3. **Ask yourself**: "How would kdapp solve this natively?"
4. **Remember the philosophy**: Framework IS the solution, not something to work around

### 🔥 REAL PRODUCTION EXAMPLE: The Challenge Fallback Bug

**On July 3, 2025, we hit this exact trap in production:**

```rust
// ❌ WRONG - This caused authentication failures!
challenge = "auth_6955901221946388822".to_string(); // Hardcoded fallback
```

**The error logs showed:**
```
WARN: Command SubmitResponse rejected: invalid command: Invalid or expired challenge.
```

**Because:**
- Server generated: `auth_9170708824197651522`
- Client used hardcoded: `auth_6955901221946388822`
- Authentication failed: challenge mismatch!

**✅ CORRECT SOLUTION:**
```rust
// Fail gracefully - no fake challenges allowed!
return Err("❌ AUTHENTICATION FAILED: Could not retrieve challenge from server.".into());
```

**EXCELLENT addition!** Those rules are GOLD for a security-critical system. Let me adapt them for the kdapp approach:

## 🔒 CRITICAL ANTI-SHORTCUT ENGINEERING GUARDS FOR KASPA AUTH

### ❌ ABSOLUTE FORBIDDEN SHORTCUTS
```rust
// ❌ NEVER DO THIS - Even for "quick testing"
fn verify_signature(pubkey: &PubKey, msg: &Message, sig: &Sig) -> bool {
    true  // "I'll implement this later" = SECURITY DISASTER
}

// ❌ NEVER DO THIS - Mock crypto is broken crypto
fn generate_challenge() -> String {
    "test_challenge_123".to_string()  // Predictable = Hackable
}

// ❌ NEVER DO THIS - Dummy auth is not auth
impl Episode for SimpleAuth {
    fn execute(&mut self, cmd: &Command, _auth: Option<PubKey>, _meta: &PayloadMetadata) -> Result<Rollback, Error> {
        self.is_authenticated = true;  // "Just to see if it compiles" = FAIL
        Ok(Rollback::Mock)
    }
}
```

### ✅ REQUIRED REAL IMPLEMENTATIONS
```rust
// ✅ CORRECT - Use real Kaspa crypto
use kaspa_consensus_core::sign::verify;
use secp256k1::{Message, PublicKey, Secp256k1, Signature};

fn verify_signature(pubkey: &PubKey, msg: &Message, sig: &Sig) -> bool {
    let secp = Secp256k1::verification_only();
    secp.verify_ecdsa(msg, &sig.0, &pubkey.0).is_ok()
}

// ✅ CORRECT - Real randomness
use rand::{thread_rng, Rng};
fn generate_challenge() -> String {
    let mut rng = thread_rng();
    format!("auth_{}", rng.gen::<u64>())
}
```

### 🎯 KDAPP-SPECIFIC GUARDS

**1. Episode Security is Blockchain Security**
```rust
// ❌ WRONG: Skipping rollback implementation
fn rollback(&mut self, _rollback: Self::CommandRollback) -> bool {
    true  // "Rollback doesn't matter for auth" = WRONG
}

// ✅ RIGHT: Every state change must be reversible
fn rollback(&mut self, rollback: AuthRollback) -> bool {
    match rollback {
        AuthRollback::Challenge(prev_challenge) => {
            self.challenge = prev_challenge;
            self.status = AuthStatus::Pending;
            true
        }
        AuthRollback::Authentication => {
            self.is_authenticated = false;
            self.session_token = None;
            true
        }
    }
}
```

**2. Use Kaspa's Existing Crypto Infrastructure**
```rust
// ✅ CORRECT: Use kaspa crates that already solved this
use kaspa_consensus_core::sign::sign_with_multiple_v2;
use kaspa_bip32::secp256k1::schnorr::Signature;

// Don't reinvent what rusty-kaspa already provides!
```

**3. Compilation ≠ Security**
```toml
# ❌ WRONG Cargo.toml - Compiles but insecure
[dependencies]
mock-crypto = "0.1"  # "Just for development" = NO

# ✅ RIGHT Cargo.toml - Real security from day 1
[dependencies]
secp256k1 = { version = "0.29", features = ["global-context", "rand-std"] }
kaspa-consensus-core = { workspace = true }
rand = "0.8"
```

### 🚨 WHEN YOU'RE TEMPTED TO SHORTCUT

**Scenario 1: "WASM won't compile with crypto"**
```rust
// ❌ WRONG: Remove crypto for WASM
#[cfg(target_arch = "wasm32")]
fn sign_message(key: &SecretKey, msg: &Message) -> Signature {
    unimplemented!("TODO: Add WASM support")
}

// ✅ RIGHT: Fix the real issue
#[cfg(target_arch = "wasm32")]
use kaspa_wasm::prelude::*;  // Use existing WASM bindings

// In Cargo.toml:
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }
kaspa-wasm = "0.15.0"
```

**Scenario 2: "Just want to test the flow"**
```rust
// ✅ RIGHT: Test with real crypto but simplified flow
#[cfg(test)]
mod tests {
    #[test]
    fn test_auth_flow() {
        // Use REAL keypairs even in tests
        let (sk, pk) = generate_keypair();
        let challenge = "real_random_challenge";
        let signature = sign_message(&sk, &to_message(&challenge));
        
        // Test with REAL verification
        assert!(verify_signature(&pk, &to_message(&challenge), &signature));
    }
}
```

### 📋 KDAPP AUTH SECURITY CHECKLIST

Before EVERY commit, verify:
- [ ] No `unimplemented!()` in security functions
- [ ] No hardcoded challenges/nonces/tokens
- [ ] All signatures use real secp256k1
- [ ] Rollback actually reverses state changes
- [ ] No `#[cfg(test)]` security bypasses
- [ ] Using kaspa's existing crypto, not reinventing

### 💭 THE MINDSET

**"If it's not secure, it's not done."**

Even for a Phase 1 demo:
- Real signatures
- Real randomness  
- Real verification
- Real rollback

The kdapp philosophy of "fastest route" doesn't mean "insecure route". It means "simplest SECURE implementation".

### 🎯 Add to CLAUDE.md:

```markdown
## 🔒 SECURITY FIRST - NO SHORTCUTS

### This is a SECURITY SYSTEM, not a toy
- Every signature must be real
- Every verification must work
- Every random value must be unpredictable
- Every rollback must restore exact previous state

### When tempted to mock/stub/dummy:
1. STOP
2. Find the existing kaspa crate that solves this
3. Use it correctly
4. If it doesn't compile, fix the real issue

### Resources for real implementations:
- `kaspa-consensus-core` - Signing and verification
- `secp256k1` - Elliptic curve operations
- `rand` - Secure randomness
- `kaspa-wasm` - WASM-compatible crypto

Remember: A broken auth system is worse than no auth system.
```

These rules will save you from the "it compiles but doesn't work" trap that kills so many crypto projects! 🔐

# Using Gemini CLI for Large Codebase Analysis

  When analyzing large codebases or multiple files that might exceed context limits, use the Gemini CLI with its massive
  context window. Use `gemini -p` to leverage Google Gemini's large context capacity.

  ## File and Directory Inclusion Syntax

  Use the `@` syntax to include files and directories in your Gemini prompts. The paths should be relative to WHERE you run the
   gemini command:

  ### Examples:

  **Single file analysis:**
  ```bash
  gemini -p "@src/main.py Explain this file's purpose and structure"

  Multiple files:
  gemini -p "@package.json @src/index.js Analyze the dependencies used in the code"

  Entire directory:
  gemini -p "@src/ Summarize the architecture of this codebase"

  Multiple directories:
  gemini -p "@src/ @tests/ Analyze test coverage for the source code"

  Current directory and subdirectories:
  gemini -p "@./ Give me an overview of this entire project"
  
#
 Or use --all_files flag:
  gemini --all_files -p "Analyze the project structure and dependencies"

  Implementation Verification Examples

  Check if a feature is implemented:
  gemini -p "@src/ @lib/ Has dark mode been implemented in this codebase? Show me the relevant files and functions"

  Verify authentication implementation:
  gemini -p "@src/ @middleware/ Is JWT authentication implemented? List all auth-related endpoints and middleware"

  Check for specific patterns:
  gemini -p "@src/ Are there any React hooks that handle WebSocket connections? List them with file paths"

  Verify error handling:
  gemini -p "@src/ @api/ Is proper error handling implemented for all API endpoints? Show examples of try-catch blocks"

  Check for rate limiting:
  gemini -p "@backend/ @middleware/ Is rate limiting implemented for the API? Show the implementation details"

  Verify caching strategy:
  gemini -p "@src/ @lib/ @services/ Is Redis caching implemented? List all cache-related functions and their usage"

  Check for specific security measures:
  gemini -p "@src/ @api/ Are SQL injection protections implemented? Show how user inputs are sanitized"

  Verify test coverage for features:
  gemini -p "@src/payment/ @tests/ Is the payment processing module fully tested? List all test cases"

  When to Use Gemini CLI

  Use gemini -p when:
  - Analyzing entire codebases or large directories
  - Comparing multiple large files
  - Need to understand project-wide patterns or architecture
  - Current context window is insufficient for the task
  - Working with files totaling more than 100KB
  - Verifying if specific features, patterns, or security measures are implemented
  - Checking for the presence of certain coding patterns across the entire codebase

  Important Notes

  - Paths in @ syntax are relative to your current working directory when invoking gemini
  - The CLI will include file contents directly in the context
  - No need for --yolo flag for read-only analysis
  - Gemini's context window can handle entire codebases that would overflow Claude's context
  - When checking implementations, be specific about what you're looking for to get accurate results # Using Gemini CLI for Large Codebase Analysis


  When analyzing large codebases or multiple files that might exceed context limits, use the Gemini CLI with its massive
  context window. Use `gemini -p` to leverage Google Gemini's large context capacity.


  ## File and Directory Inclusion Syntax


  Use the `@` syntax to include files and directories in your Gemini prompts. The paths should be relative to WHERE you run the
   gemini command:


  ### Examples:


  **Single file analysis:**
  ```bash
  gemini -p "@src/main.py Explain this file's purpose and structure"


  Multiple files:
  gemini -p "@package.json @src/index.js Analyze the dependencies used in the code"


  Entire directory:
  gemini -p "@src/ Summarize the architecture of this codebase"


  Multiple directories:
  gemini -p "@src/ @tests/ Analyze test coverage for the source code"


  Current directory and subdirectories:
  gemini -p "@./ Give me an overview of this entire project"
  # Or use --all_files flag:
  gemini --all_files -p "Analyze the project structure and dependencies"


  Implementation Verification Examples


  Check if a feature is implemented:
  gemini -p "@src/ @lib/ Has dark mode been implemented in this codebase? Show me the relevant files and functions"


  Verify authentication implementation:
  gemini -p "@src/ @middleware/ Is JWT authentication implemented? List all auth-related endpoints and middleware"


  Check for specific patterns:
  gemini -p "@src/ Are there any React hooks that handle WebSocket connections? List them with file paths"


  Verify error handling:
  gemini -p "@src/ @api/ Is proper error handling implemented for all API endpoints? Show examples of try-catch blocks"


  Check for rate limiting:
  gemini -p "@backend/ @middleware/ Is rate limiting implemented for the API? Show the implementation details"


  Verify caching strategy:
  gemini -p "@src/ @lib/ @services/ Is Redis caching implemented? List all cache-related functions and their usage"


  Check for specific security measures:
  gemini -p "@src/ @api/ Are SQL injection protections implemented? Show how user inputs are sanitized"


  Verify test coverage for features:
  gemini -p "@src/payment/ @tests/ Is the payment processing module fully tested? List all test cases"


  When to Use Gemini CLI


  Use gemini -p when:
  - Analyzing entire codebases or large directories
  - Comparing multiple large files
  - Need to understand project-wide patterns or architecture
  - Current context window is insufficient for the task
  - Working with files totaling more than 100KB
  - Verifying if specific features, patterns, or security measures are implemented
  - Checking for the presence of certain coding patterns across the entire codebase


  Important Notes


  - Paths in @ syntax are relative to your current working directory when invoking gemini
  - The CLI will include file contents directly in the context
  - No need for --yolo flag for read-only analysis
  - Gemini's context window can handle entire codebases that would overflow Claude's context
  - When checking implementations, be specific about what you're looking for to get accurate results

## 🚫 CARGO COMMANDS ARE USER RESPONSIBILITY

**CRITICAL RULE**: Claude must NEVER run cargo commands. This includes:
- ❌ `cargo build`
- ❌ `cargo run`  
- ❌ `cargo test`
- ❌ `cargo check`
- ❌ All other cargo subcommands

**Why**: 
- Compilation is the user's responsibility
- Claude should focus on code generation and architecture
- User controls when and how to build/run the project
- Avoids unnecessary token usage on compilation output

**What Claude CAN do**:
- ✅ Read/write source code files
- ✅ Analyze code structure and logic
- ✅ Suggest build commands for user to run
- ✅ Help debug compilation errors if user shares them


