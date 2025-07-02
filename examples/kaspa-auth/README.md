# Kaspa Auth Example

This example demonstrates a simple authentication episode using the `kdapp` framework.

## 🎯 Project Status

**✅ Day 1 Complete: Core Episode Structure**

- [x] `SimpleAuth` episode with challenge-response authentication
- [x] `AuthCommand` enum with `RequestChallenge` and `SubmitResponse`
- [x] Real cryptographic implementation (no shortcuts!)
- [x] Comprehensive test suite (5/5 tests passing)
- [x] Proper Rust module structure

**✅ Day 2 Complete: Kaspa Network Integration**

- [x] Episode runner with kdapp engine integration
- [x] Kaspa testnet-10 connection and transaction filtering
- [x] AUTH_PREFIX (0x41555448) for efficient transaction processing
- [x] Server/client CLI commands for real network usage
- [x] All 11 tests passing with network infrastructure ready

### Current Implementation

- **`src/simple_auth_episode.rs`** - Core authentication episode logic
- **`src/auth_commands.rs`** - Command definitions for auth flow
- **`src/episode_runner.rs`** - Kaspa network integration and episode runner
- **`src/main.rs`** - CLI interface for testing, demos, and network operations

## 🧪 Testing

Run the complete test suite:

```bash
cargo test
```

**Current Tests (11/11 Passing ✅):**
- `test_request_challenge_command` - Command creation
- `test_submit_response_command` - Response with signature/nonce
- `test_serialization` - JSON serialization/deserialization
- `test_auth_challenge_flow` - Challenge generation flow
- `test_auth_full_flow` - Complete authentication cycle
- `test_auth_pattern_uniqueness` - AUTH_PREFIX collision prevention
- `test_event_handler_creation` - Episode event handling
- `test_config_creation` - Network configuration
- `test_random_keypair_generation` - Cryptographic key generation
- `test_private_key_parsing` - Hex key parsing
- `test_episode_creation` - Episode initialization

## Usage

### Test Episode Logic

This command tests the authentication episode logic locally, without any network interaction.

```bash
cargo run -p kaspa-auth -- test-episode
```

### Interactive Demo

This command runs an interactive demo that simulates a two-party authentication flow between Alice and Bob.

```bash
cargo run -p kaspa-auth -- demo
```

### Kaspa Network Operations (Day 2)

**Run authentication server on testnet-10:**
```bash
cargo run -p kaspa-auth -- server --name "my-auth-server"
```

**Run authentication client (prepared for Day 3):**
```bash
cargo run -p kaspa-auth -- client --auth
```

**Custom private key usage:**
```bash
cargo run -p kaspa-auth -- server --key "your_hex_private_key_here"
```

## 🌐 Network Status

**Currently Connected To:** Kaspa testnet-10  
**Node:** `wss://gluon-10.kaspa.red/kaspa/testnet-10/wrpc/borsh`  
**AUTH_PREFIX:** `0x41555448` ("AUTH" in hex)  
**Pattern Filtering:** 10-point transaction pattern for efficiency

## 🚀 Next Steps (Day 3)

- [ ] Client transaction sending implementation
- [ ] Two-terminal authentication demo
- [ ] End-to-end auth flow on testnet-10
- [ ] Transaction-based challenge/response cycle

## 🔒 Security Features

- Real secp256k1 cryptography (no mocks!)
- Secure random challenge generation
- Proper signature verification
- Episode state rollback capability
