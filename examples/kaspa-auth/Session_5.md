# 🌟 Session 5: Documentation & Production Polish (Market Launch)

**Prerequisites**: Session 4 completed (poker tournament working)

## 🎯 **Session Goal: Launch-Ready Documentation**

**Why This**: Transform your working system into a professional, launch-ready product that developers can discover, understand, and integrate immediately. This is what separates demos from real products.

**Time Estimate**: 4-5 hours  
**Outcome**: Production-grade documentation, examples, and deployment guides that make kaspa-auth accessible to any developer

---

## 📋 **Phase 1: Architecture Documentation (90 minutes)**

### 1.1 Create Master README (30 min)
```markdown
<!-- README.md -->
# 🔐 Kaspa Authentication Framework

**True peer-to-peer authentication on the Kaspa blockchain**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![npm](https://img.shields.io/npm/v/@kaspa/auth-sdk)](https://www.npmjs.com/package/@kaspa/auth-sdk)

## ✨ What is Kaspa Auth?

Kaspa Auth enables **unphishable authentication** for decentralized applications. Unlike traditional authentication systems:

- **🔗 Blockchain-native**: All authentication events recorded on Kaspa
- **🚫 No servers to hack**: Participants control their own keys
- **⚡ Real-time**: WebSocket updates from blockchain events
- **🌐 Universal**: Works with any Kaspa application

## 🚀 Quick Start

### For Web Developers (REST API)
```bash
npm install @kaspa/auth-sdk
```

```javascript
import { KaspaAuth, generateKeypair, signMessage } from '@kaspa/auth-sdk';

const auth = new KaspaAuth({ endpoint: 'https://auth.kaspa.org' });
const { privateKey, publicKey } = generateKeypair();

// Authenticate
const { episodeId } = await auth.startAuthentication(publicKey);
const { nonce } = await auth.requestChallenge(episodeId);
const signature = signMessage(privateKey, nonce);
const { sessionToken } = await auth.verifySignature(episodeId, signature, nonce);

console.log('✅ Authenticated!', sessionToken);
```

### For P2P Developers (WebSocket)
```bash
npm install @kaspa/p2p-sdk
```

```javascript
import { KaspaP2P, generateKeypair, signMessage } from '@kaspa/p2p-sdk';

const p2p = new KaspaP2P({ wsEndpoint: 'ws://localhost:8080' });
await p2p.connect();

p2p.on('challenge_ready', ({ nonce }) => {
  const signature = signMessage(privateKey, nonce);
  p2p.submitSignature(episodeId, signature, nonce);
});

p2p.on('auth_success', ({ sessionToken }) => {
  console.log('✅ P2P Authentication complete!', sessionToken);
});
```

## 🏗️ Architecture

Kaspa Auth implements a **hybrid P2P architecture**:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Participant   │◄──►│ HTTP Organizer  │◄──►│ Kaspa Blockchain│
│   (Browser)     │    │ Peer (Coord.)   │    │ (Source of Truth│
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        │                        │                        │
        ▼                        ▼                        ▼
   WebSocket              kdapp Engine              Transaction
   Real-time             Episode Runner              Validation
```

### Two Deployment Options:

1. **Managed Service**: Use hosted coordinator (like Firebase)
2. **Self-Hosted**: Run your own coordinator for full decentralization

## 📖 Documentation

- **[Quick Start Guide](docs/quickstart.md)** - 5 minute integration
- **[Architecture Overview](docs/architecture.md)** - Understanding the P2P model
- **[API Reference](docs/api-reference.md)** - Complete endpoint documentation
- **[Self-Hosting Guide](docs/self-hosting.md)** - Run your own coordinator
- **[Security Model](docs/security.md)** - Cryptographic guarantees

## 🎯 Examples

- **[Basic Web Auth](examples/web-auth/)** - Simple login/logout
- **[React Integration](examples/react-app/)** - Modern React app
- **[Pure P2P Demo](examples/pure-p2p/)** - WebSocket-first approach
- **[Poker Tournament](examples/kaspa-poker/)** - Complex multi-participant app

## 🛠️ Building from Source

```bash
git clone https://github.com/kaspa/kaspa-auth
cd kaspa-auth

# Run coordinator
cargo run -- http-peer

# Or pure P2P server
cd examples/pure-p2p
cargo run
```

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md)

## 📜 License

MIT License - see [LICENSE](LICENSE)

---

**Built with [kdapp](https://github.com/michaelsutton/kdapp) - Kaspa Decentralized Application Framework**
```

### 1.2 Architecture Deep-Dive (30 min)
```markdown
<!-- docs/architecture.md -->
# 🏗️ Kaspa Auth Architecture

## Overview: True P2P Authentication

Kaspa Auth breaks the traditional client-server authentication model by implementing **true peer-to-peer authentication** on the Kaspa blockchain.

## The Problem with Traditional Auth

```
❌ Traditional: Client → Server → Database
   - Server can be hacked
   - Passwords can be stolen  
   - Single point of failure
   - Vendor lock-in
```

## The Kaspa Auth Solution

```
✅ P2P: Participant ↔ Coordinator ↔ Blockchain
   - No passwords stored anywhere
   - Unphishable cryptographic proof
   - Participants control their keys
   - Coordinator is replaceable
```

## Core Components

### 1. Authentication Episode (Rust)
**Location**: `src/core/episode.rs` (200 lines)

The heart of the system - a kdapp episode that implements challenge-response authentication:

```rust
pub struct SimpleAuth {
    pub challenge: Option<String>,
    pub is_authenticated: bool,
    pub session_token: Option<String>,
    pub authenticated_key: Option<PubKey>,
    pub created_at: SystemTime,
}
```

**Commands:**
- `RequestChallenge` - Generate cryptographic challenge
- `SubmitResponse` - Verify signed challenge
- `RevokeSession` - Blockchain-based logout

### 2. HTTP Organizer Peer (Coordination)
**Location**: `src/api/http/` (~1200 lines)

Provides familiar HTTP/WebSocket interface for web developers:

- **REST API**: Start auth, get challenge, verify signature
- **WebSocket**: Real-time blockchain event notifications
- **Transaction Relay**: Submit participant transactions to blockchain

**Key Insight**: This is **coordination**, not **control**. The blockchain remains the source of truth.

### 3. Participant Interfaces

#### Web Dashboard (`public/index.html`)
- Modern browser interface
- WebSocket real-time updates
- Local key management
- Session token handling

#### CLI (`src/main.rs`)
- Command-line authentication
- Hybrid kdapp + HTTP coordination
- Persistent wallet management

#### NPM SDKs (`sdk/`)
- `@kaspa/auth-sdk` - REST API client for web developers
- `@kaspa/p2p-sdk` - WebSocket client for P2P applications

## Authentication Flow

### Phase 1: Challenge Request
```mermaid
sequenceDiagram
    participant P as Participant
    participant C as Coordinator
    participant B as Blockchain
    
    P->>C: POST /auth/start {public_key}
    C->>B: Submit RequestChallenge transaction
    B->>C: Transaction confirmed
    C->>P: WebSocket: challenge_ready {nonce}
```

### Phase 2: Challenge Response
```mermaid
sequenceDiagram
    participant P as Participant
    participant C as Coordinator
    participant B as Blockchain
    
    P->>P: Sign challenge with private key
    P->>C: POST /auth/verify {signature}
    C->>B: Submit SubmitResponse transaction
    B->>C: Transaction confirmed
    C->>P: WebSocket: auth_success {session_token}
```

### Phase 3: Session Management
```mermaid
sequenceDiagram
    participant P as Participant
    participant A as Application
    participant C as Coordinator
    participant B as Blockchain
    
    P->>A: Use app with session_token
    A->>C: Verify session validity
    C->>A: Session valid/invalid
    
    Note over P,B: Later: Logout
    P->>C: POST /auth/revoke-session
    C->>B: Submit RevokeSession transaction
    B->>C: Transaction confirmed
    C->>P: WebSocket: session_revoked
```

## Security Properties

### Cryptographic Guarantees
- **Challenge Uniqueness**: Each challenge is cryptographically random
- **Signature Verification**: secp256k1 ECDSA signatures (same as Bitcoin)
- **Public Key Authentication**: Prove ownership without revealing private key
- **Session Tokens**: Time-limited, revocable identifiers

### Attack Resistance
- **Phishing**: Impossible - no passwords to steal
- **Server Compromise**: Coordinator has no secrets to steal
- **Replay Attacks**: Each challenge is unique and single-use
- **Man-in-the-Middle**: Signatures cryptographically bound to specific challenges

### Blockchain Properties
- **Immutable Audit Trail**: All auth events permanently recorded
- **Decentralized Verification**: Any participant can verify authentication history
- **Censorship Resistance**: No single party can block authentication
- **Global State**: All participants see same authentication state

## Deployment Models

### 1. Managed Service (Recommended for Most)
```
Web App → https://auth.kaspa.org → Kaspa Blockchain
```
- Easy integration (like Firebase Auth)
- Professional SLA and support
- No infrastructure management
- Still cryptographically secure

### 2. Self-Hosted (Maximum Decentralization)
```
Web App → Your Coordinator → Kaspa Blockchain
```
- Full control and privacy
- Custom business logic
- No vendor dependency
- Requires technical expertise

### 3. Pure P2P (Advanced Use Cases)
```
P2P App ↔ WebSocket ↔ kdapp Engine ↔ Kaspa Blockchain
```
- No HTTP layer at all
- Maximum decentralization
- Real-time blockchain events
- For sophisticated P2P applications

## Comparison with Alternatives

| Feature | Kaspa Auth | OAuth2/OIDC | Blockchain Wallets | Enterprise SSO |
|---------|------------|-------------|-------------------|----------------|
| **Phishing Resistance** | ✅ Impossible | ❌ Common target | ✅ Cryptographic | ❌ Password-based |
| **Server Dependency** | ⚠️ Coordinator only | ❌ Auth server required | ✅ None | ❌ Multiple servers |
| **User Experience** | ✅ Familiar web flow | ✅ Very familiar | ❌ Complex setup | ✅ SSO convenience |
| **Developer Integration** | ✅ REST API + SDK | ✅ Standard protocols | ❌ Wallet-specific | ✅ Standard protocols |
| **Censorship Resistance** | ✅ Blockchain-based | ❌ Server-controlled | ✅ Decentralized | ❌ Corporate-controlled |
| **Audit Trail** | ✅ Immutable | ⚠️ Server logs | ⚠️ Wallet-dependent | ⚠️ Enterprise logs |

## Performance Characteristics

- **Authentication Latency**: ~2-5 seconds (blockchain confirmation)
- **Session Validation**: <100ms (local verification)
- **Throughput**: Limited by Kaspa transaction capacity
- **Storage**: Minimal - only session tokens cached
- **Scalability**: Horizontal via multiple coordinators

## Future Roadmap

### Near Term (Months)
- [ ] Production-hardened coordinator implementation
- [ ] Additional language SDKs (Python, Go, Java)
- [ ] Enterprise integration guides
- [ ] Performance optimizations

### Medium Term (Quarters)
- [ ] Multi-signature authentication support
- [ ] Hardware wallet integration
- [ ] Mobile app SDK
- [ ] Formal security audit

### Long Term (Years)
- [ ] Cross-chain authentication bridges
- [ ] Zero-knowledge proof integration
- [ ] Quantum-resistant cryptography
- [ ] Standardization as IETF RFC

---

*This architecture enables applications like decentralized poker tournaments, P2P marketplaces, and DAO governance - all with unphishable authentication.*
```

### 1.3 Security Documentation (30 min)
```markdown
<!-- docs/security.md -->
# 🔒 Security Model

## Threat Model

### What Kaspa Auth Protects Against

✅ **Phishing Attacks**
- No passwords to steal
- Private keys never leave user's device
- Challenge-response proves key ownership

✅ **Server Compromise**
- Coordinator stores no secrets
- Session tokens are time-limited
- Blockchain remains authoritative

✅ **Replay Attacks**
- Each challenge is cryptographically unique
- Nonces prevent signature reuse
- Timestamps prevent stale challenges

✅ **Man-in-the-Middle**
- Signatures cryptographically bound to specific challenges
- TLS protects coordination layer
- Blockchain provides immutable verification

### What Kaspa Auth Does NOT Protect Against

❌ **Device Compromise**
- If user's device is compromised, private keys can be stolen
- Mitigation: Use hardware wallets for high-value applications

❌ **Social Engineering**
- Users can still be tricked into signing malicious transactions
- Mitigation: Clear transaction display and user education

❌ **Quantum Computing**
- secp256k1 is vulnerable to sufficient quantum computers
- Mitigation: Quantum-resistant algorithms in future versions

## Cryptographic Primitives

### Key Generation
- **Algorithm**: secp256k1 (same as Bitcoin/Ethereum)
- **Key Size**: 256-bit private keys
- **Entropy**: Cryptographically secure random number generation
- **Address Format**: Kaspa bech32 encoding

### Digital Signatures
- **Algorithm**: ECDSA with secp256k1 curve
- **Hash Function**: SHA-256
- **Signature Format**: DER encoding
- **Verification**: Public key recovery from signature

### Challenge Generation
```rust
pub fn generate_challenge() -> String {
    let mut rng = thread_rng();
    let nonce: u64 = rng.gen();
    format!("auth_{}", nonce)
}
```

### Session Tokens
- **Format**: Base64-encoded random bytes
- **Length**: 256 bits (44 characters)
- **Lifetime**: 1 hour (configurable)
- **Revocation**: Blockchain-based for global consistency

## Implementation Security

### Private Key Management
```javascript
// ✅ CORRECT: Generate keys securely
const { privateKey, publicKey } = generateKeypair();
localStorage.setItem('kaspa_private_key', Buffer.from(privateKey).toString('hex'));

// ❌ WRONG: Weak key generation
const privateKey = "0123456789abcdef..."; // Predictable
```

### Signature Verification
```rust
// ✅ CORRECT: Verify all signature components
pub fn verify_signature(public_key: &PubKey, message: &str, signature: &str) -> bool {
    let msg_hash = sha256(message.as_bytes());
    let sig_bytes = hex::decode(signature).unwrap();
    secp256k1::verify(&msg_hash, &sig_bytes, &public_key.0)
}

// ❌ WRONG: Skip verification
pub fn verify_signature_unsafe(_: &PubKey, _: &str, _: &str) -> bool {
    true // Never do this!
}
```

### Session Token Validation
```javascript
// ✅ CORRECT: Check expiry and signature
function validateSession(token, publicKey) {
    const session = sessions.get(token);
    if (!session || session.expiresAt < Date.now()) {
        return false;
    }
    return session.publicKey === publicKey;
}

// ❌ WRONG: No expiry check
function validateSessionUnsafe(token) {
    return sessions.has(token);
}
```

## Coordinator Security

### Input Validation
- All API inputs validated against strict schemas
- Rate limiting on authentication attempts
- CORS protection for web applications
- Content-Type validation for JSON APIs

### Error Handling
```rust
// ✅ CORRECT: Don't leak information
match verify_signature(&pubkey, &challenge, &signature) {
    Ok(true) => Ok("Authentication successful"),
    _ => Err("Authentication failed"), // Generic error
}

// ❌ WRONG: Information leakage
match verify_signature(&pubkey, &challenge, &signature) {
    Ok(true) => Ok("Authentication successful"),
    Ok(false) => Err("Invalid signature"), // Reveals signature was checked
    Err(e) => Err(format!("Verification error: {}", e)), // Reveals internal state
}
```

### WebSocket Security
- Origin validation for WebSocket connections
- Message size limits to prevent DoS
- Rate limiting on message frequency
- Automatic disconnection on invalid messages

## Deployment Security

### TLS Configuration
```nginx
# ✅ Recommended nginx config
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
ssl_prefer_server_ciphers off;
ssl_session_cache shared:SSL:10m;
```

### Environment Variables
```bash
# ✅ CORRECT: Use environment variables for secrets
export KASPA_NETWORK="testnet-10"
export KASPA_COORDINATOR_PORT="8080"
export LOG_LEVEL="info"

# ❌ WRONG: Hardcode in source code
const KASPA_NETWORK = "testnet-10"; // Don't commit networks/keys
```

### Monitoring and Alerting
- Authentication failure rate monitoring
- Unusual transaction pattern detection
- Session token abuse detection
- Blockchain reorganization alerts

## Security Audit Checklist

### Pre-Production
- [ ] All dependencies audited for vulnerabilities
- [ ] No hardcoded secrets in source code
- [ ] Rate limiting implemented and tested
- [ ] Input validation comprehensive
- [ ] Error messages don't leak information
- [ ] TLS configuration follows best practices
- [ ] Private keys never logged or transmitted
- [ ] Session tokens have appropriate lifetimes
- [ ] Blockchain transaction validation correct

### Production
- [ ] Security monitoring in place
- [ ] Incident response plan defined
- [ ] Backup and recovery procedures tested
- [ ] Penetration testing completed
- [ ] Code review by security expert
- [ ] Formal security audit (for high-value applications)

## Incident Response

### Security Incident Types

**Level 1: Low Impact**
- Single user authentication failure
- Temporary coordinator unavailability
- Minor configuration issues

**Level 2: Medium Impact**
- Multiple user authentication failures
- Coordinator performance degradation
- Blockchain network issues

**Level 3: High Impact**
- Coordinator compromise attempt
- Large-scale authentication failures
- Potential cryptographic issues

### Response Procedures

1. **Detection**: Automated monitoring alerts
2. **Assessment**: Determine impact and scope
3. **Containment**: Isolate affected systems
4. **Recovery**: Restore normal operations
5. **Lessons Learned**: Update procedures and defenses

---

*Security is a process, not a destination. Regular audits and updates are essential for maintaining the security of Kaspa Auth deployments.*
```

---

## 📋 **Phase 2: Developer Experience (75 minutes)**

### 2.1 API Reference Documentation (30 min)
```markdown
<!-- docs/api-reference.md -->
# 📚 API Reference

## REST API Endpoints

### Authentication Lifecycle

#### Start Authentication
```http
POST /auth/start
Content-Type: application/json

{
    "public_key": "kaspatest:qz9x8y7w6v5u4t3s2r1q0p..."
}
```

**Response:**
```json
{
    "episode_id": 1234567890,
    "status": "challenge_pending"
}
```

#### Request Challenge
```http
GET /auth/challenge/{episode_id}
```

**Response:**
```json
{
    "episode_id": 1234567890,
    "nonce": "auth_9876543210",
    "expires_at": "2024-01-01T12:00:00Z"
}
```

#### Verify Signature
```http
POST /auth/verify
Content-Type: application/json

{
    "episode_id": 1234567890,
    "signature": "3045022100a1b2c3...",
    "nonce": "auth_9876543210"
}
```

**Response:**
```json
{
    "episode_id": 1234567890,
    "session_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_at": "2024-01-01T13:00:00Z",
    "public_key": "kaspatest:qz9x8y7w6v5u4t3s2r1q0p..."
}
```

#### Revoke Session
```http
POST /auth/revoke-session
Content-Type: application/json

{
    "episode_id": 1234567890,
    "session_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Response:**
```json
{
    "transaction_id": "abc123def456...",
    "status": "session_revocation_submitted"
}
```

### Session Management

#### Check Session Status
```http
GET /auth/status/{episode_id}
```

**Response:**
```json
{
    "episode_id": 1234567890,
    "status": "authenticated",
    "expires_at": "2024-01-01T13:00:00Z",
    "session_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

#### Refresh Session (if implemented)
```http
POST /auth/refresh
Content-Type: application/json

{
    "session_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

## WebSocket Events

### Connection
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');
```

### Outgoing Events (Client → Server)
```javascript
// Subscribe to episode updates
ws.send(JSON.stringify({
    type: 'subscribe',
    episode_id: 1234567890
}));
```

### Incoming Events (Server → Client)

#### Challenge Ready
```json
{
    "type": "challenge_ready",
    "episode_id": 1234567890,
    "nonce": "auth_9876543210",
    "expires_at": "2024-01-01T12:00:00Z"
}
```

#### Authentication Success
```json
{
    "type": "auth_success",
    "episode_id": 1234567890,
    "session_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_at": "2024-01-01T13:00:00Z"
}
```

#### Authentication Failed
```json
{
    "type": "auth_failed",
    "episode_id": 1234567890,
    "reason": "Invalid signature"
}
```

#### Session Revoked
```json
{
    "type": "session_revoked",
    "episode_id": 1234567890,
    "transaction_id": "abc123def456..."
}
```

## SDK Documentation

### @kaspa/auth-sdk (REST Client)

#### Installation
```bash
npm install @kaspa/auth-sdk
```

#### Basic Usage
```javascript
import { KaspaAuth, generateKeypair, signMessage } from '@kaspa/auth-sdk';

const auth = new KaspaAuth({
    endpoint: 'https://auth.kaspa.org',
    network: 'testnet-10',
    timeout: 30000
});

// Generate keypair
const { privateKey, publicKey } = generateKeypair();

// Full authentication flow
async function authenticate() {
    try {
        // 1. Start authentication
        const { episodeId } = await auth.startAuthentication(publicKey);
        console.log('Episode ID:', episodeId);
        
        // 2. Get challenge
        const { nonce } = await auth.requestChallenge(episodeId);
        console.log('Challenge:', nonce);
        
        // 3. Sign challenge
        const signature = signMessage(privateKey, nonce);
        console.log('Signature:', signature);
        
        // 4. Verify signature
        const result = await auth.verifySignature(episodeId, signature, nonce);
        console.log('✅ Authenticated!', result.sessionToken);
        
        return result;
    } catch (error) {
        console.error('❌ Authentication failed:', error);
        throw error;
    }
}
```

#### Configuration Options
```javascript
const auth = new KaspaAuth({
    endpoint: 'https://auth.kaspa.org',  // Required
    network: 'testnet-10',               // Optional, default: 'testnet-10'
    timeout: 30000,                      // Optional, default: 30000ms
    retries: 3,                          // Optional, default: 3
    retryDelay: 1000                     // Optional, default: 1000ms
});
```

### @kaspa/p2p-sdk (WebSocket Client)

#### Installation
```bash
npm install @kaspa/p2p-sdk
```

#### Basic Usage
```javascript
import { KaspaP2P, generateKeypair, signMessage } from '@kaspa/p2p-sdk';

const p2p = new KaspaP2P({
    wsEndpoint: 'ws://localhost:8080',
    reconnectInterval: 5000,
    timeout: 30000
});

// Event-driven authentication
async function authenticateP2P() {
    const { privateKey, publicKey } = generateKeypair();
    
    // Connect to P2P network
    await p2p.connect();
    
    // Set up event handlers
    p2p.on('challenge_ready', ({ episodeId, nonce }) => {
        console.log('Challenge received:', nonce);
        const signature = signMessage(privateKey, nonce);
        p2p.submitSignature(episodeId, signature, nonce);
    });
    
    p2p.on('auth_success', ({ sessionToken }) => {
        console.log('✅ P2P Authentication successful!', sessionToken);
    });
    
    p2p.on('auth_failed', ({ reason }) => {
        console.error('❌ P2P Authentication failed:', reason);
    });
    
    // Start authentication
    const episodeId = await p2p.requestAuth(publicKey);
    console.log('Started P2P authentication, episode:', episodeId);
}
```

## Error Handling

### HTTP Status Codes
- `200 OK` - Success
- `400 Bad Request` - Invalid request format
- `401 Unauthorized` - Authentication failed
- `404 Not Found` - Episode not found
- `409 Conflict` - Episode already exists
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

### Error Response Format
```json
{
    "error": {
        "code": "INVALID_SIGNATURE",
        "message": "The provided signature is invalid",
        "episode_id": 1234567890,
        "timestamp": "2024-01-01T12:00:00Z"
    }
}
```

### Common Error Codes
- `INVALID_PUBLIC_KEY` - Malformed public key
- `INVALID_SIGNATURE` - Signature verification failed
- `CHALLENGE_EXPIRED` - Challenge nonce expired
- `EPISODE_NOT_FOUND` - Episode ID not found
- `SESSION_EXPIRED` - Session token expired
- `RATE_LIMITED` - Too many requests

## Rate Limiting

### Default Limits
- Authentication attempts: 5 per minute per IP
- Challenge requests: 10 per minute per IP  
- Session verification: 100 per minute per IP
- WebSocket connections: 10 per IP

### Rate Limit Headers
```http
X-RateLimit-Limit: 5
X-RateLimit-Remaining: 3
X-RateLimit-Reset: 1640995200
```

---

*For more examples and advanced usage, see the [examples](../examples/) directory.*
```

### 2.2 Quick Start Guide (25 min)
```markdown
<!-- docs/quickstart.md -->
# 🚀 Quick Start Guide

Get Kaspa authentication working in your app in 5 minutes!

## Choose Your Path

### 🌐 Web Developer (REST API)
Perfect for existing web applications, React apps, and traditional backends.

### ⚡ P2P Developer (WebSocket)
For real-time applications, P2P games, and advanced blockchain integrations.

---

## 🌐 Web Developer Path

### Step 1: Install SDK
```bash
npm install @kaspa/auth-sdk
```

### Step 2: Basic Integration
```javascript
import { KaspaAuth, generateKeypair, signMessage } from '@kaspa/auth-sdk';

// Initialize auth client
const auth = new KaspaAuth({
    endpoint: 'https://auth.kaspa.org'  // Use hosted service
});

// Generate user keypair (do this once per user)
const { privateKey, publicKey } = generateKeypair();

// Store safely (production: use secure storage)
localStorage.setItem('kaspa_private_key', privateKey);
localStorage.setItem('kaspa_public_key', publicKey);
```

### Step 3: Add Login Button
```html
<button id="login-btn">🔐 Login with Kaspa</button>
<div id="auth-status"></div>

<script>
document.getElementById('login-btn').onclick = async () => {
    try {
        const status = document.getElementById('auth-status');
        status.textContent = '🔄 Authenticating...';
        
        // Get stored keys
        const privateKey = localStorage.getItem('kaspa_private_key');
        const publicKey = localStorage.getItem('kaspa_public_key');
        
        // Authenticate
        const { episodeId } = await auth.startAuthentication(publicKey);
        const { nonce } = await auth.requestChallenge(episodeId);
        const signature = signMessage(privateKey, nonce);
        const { sessionToken } = await auth.verifySignature(episodeId, signature, nonce);
        
        // Success!
        status.textContent = '✅ Authenticated!';
        localStorage.setItem('kaspa_session', sessionToken);
        
        // Now you can use sessionToken for API calls
        console.log('Session token:', sessionToken);
        
    } catch (error) {
        document.getElementById('auth-status').textContent = '❌ Login failed';
        console.error(error);
    }
};
</script>
```

### Step 4: Protect Your APIs
```javascript
// Backend: Verify session tokens
async function verifyKaspaAuth(sessionToken) {
    try {
        const response = await fetch('https://auth.kaspa.org/auth/verify-session', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ session_token: sessionToken })
        });
        
        return response.ok;
    } catch {
        return false;
    }
}

// Express.js middleware example
app.use('/api/protected', async (req, res, next) => {
    const sessionToken = req.headers.authorization?.replace('Bearer ', '');
    
    if (!sessionToken || !await verifyKaspaAuth(sessionToken)) {
        return res.status(401).json({ error: 'Authentication required' });
    }
    
    next();
});
```

### Step 5: Add Logout
```javascript
async function logout() {
    const sessionToken = localStorage.getItem('kaspa_session');
    const episodeId = localStorage.getItem('kaspa_episode_id');
    
    if (sessionToken && episodeId) {
        try {
            await auth.revokeSession(episodeId, sessionToken);
            console.log('✅ Logged out from blockchain');
        } catch (error) {
            console.warn('Logout warning:', error);
        }
    }
    
    // Clear local session
    localStorage.removeItem('kaspa_session');
    localStorage.removeItem('kaspa_episode_id');
    
    document.getElementById('auth-status').textContent = 'Logged out';
}
```

### 🎉 Done! You now have unphishable authentication!

---

## ⚡ P2P Developer Path

### Step 1: Install P2P SDK
```bash
npm install @kaspa/p2p-sdk
```

### Step 2: Connect to P2P Network
```javascript
import { KaspaP2P, generateKeypair, signMessage } from '@kaspa/p2p-sdk';

// Connect to P2P auth network
const p2p = new KaspaP2P({
    wsEndpoint: 'ws://localhost:8080'  // Run your own node
});

await p2p.connect();
console.log('🟢 Connected to Kaspa P2P auth network');
```

### Step 3: Event-Driven Authentication
```javascript
const { privateKey, publicKey } = generateKeypair();
let currentSession = null;

// Set up event handlers
p2p.on('connected', () => {
    console.log('🟢 P2P network connected');
});

p2p.on('challenge_ready', ({ episodeId, nonce }) => {
    console.log('🎲 Challenge received:', nonce);
    
    // Sign the challenge
    const signature = signMessage(privateKey, nonce);
    
    // Submit response
    p2p.submitSignature(episodeId, signature, nonce);
    console.log('✍️ Signature submitted');
});

p2p.on('auth_success', ({ episodeId, sessionToken }) => {
    console.log('✅ Authentication successful!');
    currentSession = { episodeId, sessionToken };
    
    // Update UI
    document.getElementById('status').textContent = '✅ Authenticated via P2P!';
    document.getElementById('session-info').textContent = sessionToken;
});

p2p.on('auth_failed', ({ reason }) => {
    console.error('❌ Authentication failed:', reason);
    document.getElementById('status').textContent = '❌ Authentication failed';
});

// Start authentication
const episodeId = await p2p.requestAuth(publicKey);
console.log('🚀 Authentication started, episode:', episodeId);
```

### Step 4: Use in Your P2P App
```javascript
// Example: P2P game authentication
class P2PGame {
    constructor() {
        this.auth = new KaspaP2P({ wsEndpoint: 'ws://game.example.com' });
        this.gameSession = null;
    }
    
    async joinGame(gameId) {
        // Must be authenticated first
        if (!currentSession) {
            throw new Error('Authentication required');
        }
        
        // Join game with authenticated session
        this.auth.send({
            type: 'join_game',
            game_id: gameId,
            session_token: currentSession.sessionToken,
            public_key: publicKey
        });
    }
}
```

### Step 5: P2P Logout
```javascript
async function p2pLogout() {
    if (currentSession) {
        // Revoke session on blockchain
        await p2p.revokeSession(currentSession.episodeId, currentSession.sessionToken);
        console.log('🚪 Session revoked on blockchain');
        
        currentSession = null;
        document.getElementById('status').textContent = 'Logged out';
    }
}
```

### 🎉 Done! You have real-time P2P authentication!

---

## 🚀 Next Steps

### Production Checklist
- [ ] **Security**: Use HTTPS in production
- [ ] **Keys**: Implement secure key storage (hardware wallets for high value)
- [ ] **Monitoring**: Add authentication analytics
- [ ] **Fallbacks**: Handle network failures gracefully
- [ ] **UX**: Add loading states and error messages

### Advanced Features
- [ ] **Self-Hosting**: Run your own coordinator for maximum decentralization
- [ ] **Multi-Signature**: Support multi-sig authentication
- [ ] **Session Management**: Implement session refresh and management
- [ ] **Integration**: Connect with existing user systems

### Examples to Explore
- **[React App](../examples/react-app/)** - Modern React integration
- **[Poker Game](../examples/kaspa-poker/)** - Complex P2P application
- **[Pure P2P](../examples/pure-p2p/)** - WebSocket-first architecture

### Get Help
- **[GitHub Issues](https://github.com/kaspa/kaspa-auth/issues)** - Bug reports and questions
- **[Discord](https://discord.gg/kaspa)** - Community support
- **[Docs](https://docs.kaspa.org/auth)** - Complete documentation

---

**Welcome to unphishable authentication! 🔐**
```

### 2.3 Self-Hosting Guide (20 min)
```markdown
<!-- docs/self-hosting.md -->
# 🏠 Self-Hosting Guide

Run your own Kaspa Auth coordinator for maximum decentralization and control.

## Why Self-Host?

- **🔒 Privacy**: No third-party dependency
- **🎛️ Control**: Custom configuration and business logic
- **🌐 Decentralization**: Reduce single points of failure
- **💰 Cost**: No service fees for high volume

## Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs)
- **Kaspa Node**: Access to testnet-10 or mainnet node
- **Domain**: For production deployment (optional for testing)
- **TLS Certificate**: For HTTPS (Let's Encrypt recommended)

## Quick Start (Local Development)

### 1. Clone and Build
```bash
git clone https://github.com/kaspa/kaspa-auth
cd kaspa-auth

# Build the coordinator
cargo build --release

# Copy example config
cp config/example.toml config/local.toml
```

### 2. Configure
```toml
# config/local.toml
[network]
name = "testnet-10"
kaspad_url = "127.0.0.1:16210"  # Your local kaspad

[server]
host = "127.0.0.1"
port = 8080
tls_enabled = false  # OK for local development

[auth]
session_lifetime = 3600  # 1 hour
max_auth_attempts = 5
rate_limit_window = 60

[logging]
level = "info"
file = "kaspa-auth.log"
```

### 3. Run
```bash
# Start coordinator
cargo run --release -- http-peer --config config/local.toml

# Or with environment variables
KASPA_NETWORK=testnet-10 \
KASPA_PORT=8080 \
RUST_LOG=info \
cargo run --release -- http-peer
```

### 4. Test
```bash
# Test health endpoint
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","network":"testnet-10","uptime":42}
```

## Production Deployment

### Option 1: Docker (Recommended)

#### Build Docker Image
```dockerfile
# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kaspa-auth /usr/local/bin/
COPY --from=builder /app/public /app/public
WORKDIR /app
EXPOSE 8080
CMD ["kaspa-auth", "http-peer"]
```

#### Docker Compose
```yaml
# docker-compose.yml
version: '3.8'
services:
  kaspa-auth:
    build: .
    ports:
      - "8080:8080"
    environment:
      - KASPA_NETWORK=testnet-10
      - KASPA_KASPAD_URL=kaspad.example.com:16210
      - RUST_LOG=info
    volumes:
      - ./config:/app/config:ro
      - ./logs:/app/logs
    restart: unless-stopped
    
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - kaspa-auth
    restart: unless-stopped
```

#### Run with Docker
```bash
# Build and start
docker-compose up -d

# Check logs
docker-compose logs -f kaspa-auth

# Update
git pull
docker-compose build
docker-compose up -d
```

### Option 2: systemd Service

#### Create Service File
```ini
# /etc/systemd/system/kaspa-auth.service
[Unit]
Description=Kaspa Authentication Coordinator
After=network.target

[Service]
Type=simple
User=kaspa-auth
Group=kaspa-auth
WorkingDirectory=/opt/kaspa-auth
ExecStart=/opt/kaspa-auth/kaspa-auth http-peer --config /opt/kaspa-auth/config/production.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/kaspa-auth/logs
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

#### Deploy
```bash
# Create user
sudo useradd --system --home /opt/kaspa-auth kaspa-auth

# Deploy files
sudo mkdir -p /opt/kaspa-auth/{config,logs}
sudo cp target/release/kaspa-auth /opt/kaspa-auth/
sudo cp -r public /opt/kaspa-auth/
sudo cp config/production.toml /opt/kaspa-auth/config/
sudo chown -R kaspa-auth:kaspa-auth /opt/kaspa-auth

# Enable and start service
sudo systemctl enable kaspa-auth
sudo systemctl start kaspa-auth

# Check status
sudo systemctl status kaspa-auth
sudo journalctl -u kaspa-auth -f
```

## Nginx Configuration

### TLS Termination
```nginx
# nginx.conf
events {
    worker_connections 1024;
}

http {
    # Rate limiting
    limit_req_zone $binary_remote_addr zone=auth:10m rate=10r/m;
    
    # Upstream
    upstream kaspa-auth {
        server 127.0.0.1:8080;
    }
    
    # Redirect HTTP to HTTPS
    server {
        listen 80;
        server_name auth.example.com;
        return 301 https://$server_name$request_uri;
    }
    
    # HTTPS server
    server {
        listen 443 ssl http2;
        server_name auth.example.com;
        
        # TLS configuration
        ssl_certificate /etc/nginx/ssl/fullchain.pem;
        ssl_certificate_key /etc/nginx/ssl/privkey.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
        ssl_prefer_server_ciphers off;
        
        # Security headers
        add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
        add_header X-Frame-Options DENY always;
        add_header X-Content-Type-Options nosniff always;
        add_header Referrer-Policy strict-origin-when-cross-origin always;
        
        # API endpoints
        location /auth/ {
            limit_req zone=auth burst=20 nodelay;
            proxy_pass http://kaspa-auth;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
        
        # WebSocket upgrade
        location /ws {
            proxy_pass http://kaspa-auth;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
        }
        
        # Static files
        location / {
            proxy_pass http://kaspa-auth;
            proxy_set_header Host $host;
        }
    }
}
```

## Monitoring and Maintenance

### Health Checks
```bash
#!/bin/bash
# health-check.sh
ENDPOINT="https://auth.example.com/health"
EXPECTED_STATUS="healthy"

response=$(curl -s "$ENDPOINT" | jq -r '.status')

if [ "$response" = "$EXPECTED_STATUS" ]; then
    echo "✅ Health check passed"
    exit 0
else
    echo "❌ Health check failed: $response"
    exit 1
fi
```

### Monitoring Script
```bash
#!/bin/bash
# monitor.sh
while true; do
    # Check coordinator health
    if ! ./health-check.sh; then
        echo "⚠️ Coordinator unhealthy, restarting..."
        sudo systemctl restart kaspa-auth
    fi
    
    # Check disk space
    DISK_USAGE=$(df /opt/kaspa-auth | tail -1 | awk '{print $5}' | sed 's/%//')
    if [ "$DISK_USAGE" -gt 80 ]; then
        echo "⚠️ Disk usage high: ${DISK_USAGE}%"
        # Rotate logs
        sudo logrotate /etc/logrotate.d/kaspa-auth
    fi
    
    # Check memory usage
    MEM_USAGE=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}')
    if [ "$MEM_USAGE" -gt 90 ]; then
        echo "⚠️ Memory usage high: ${MEM_USAGE}%"
    fi
    
    sleep 300  # Check every 5 minutes
done
```

### Log Rotation
```
# /etc/logrotate.d/kaspa-auth
/opt/kaspa-auth/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    postrotate
        systemctl reload kaspa-auth
    endscript
}
```

## Backup and Recovery

### Configuration Backup
```bash
#!/bin/bash
# backup-config.sh
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backup/kaspa-auth"

mkdir -p "$BACKUP_DIR"

# Backup configuration
tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" -C /opt/kaspa-auth config/

# Backup logs (optional)
tar -czf "$BACKUP_DIR/logs_$DATE.tar.gz" -C /opt/kaspa-auth logs/

# Keep only last 30 days
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +30 -delete

echo "✅ Backup completed: $BACKUP_DIR"
```

### Disaster Recovery
```bash
#!/bin/bash
# restore.sh
BACKUP_FILE="$1"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.tar.gz>"
    exit 1
fi

# Stop service
sudo systemctl stop kaspa-auth

# Restore configuration
sudo tar -xzf "$BACKUP_FILE" -C /opt/kaspa-auth/

# Fix permissions
sudo chown -R kaspa-auth:kaspa-auth /opt/kaspa-auth

# Start service
sudo systemctl start kaspa-auth

echo "✅ Restore completed"
```

## Security Considerations

### Firewall Configuration
```bash
# UFW example
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 80/tcp    # HTTP (redirect)
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable

# Block direct access to coordinator
sudo ufw deny 8080/tcp
```

### Fail2Ban Protection
```ini
# /etc/fail2ban/jail.local
[kaspa-auth]
enabled = true
port = 443
filter = kaspa-auth
logpath = /opt/kaspa-auth/logs/kaspa-auth.log
maxretry = 5
bantime = 3600
```

```
# /etc/fail2ban/filter.d/kaspa-auth.conf
[Definition]
failregex = Authentication failed.*from.*<HOST>
ignoreregex =
```

### Regular Updates
```bash
#!/bin/bash
# update.sh
cd /opt/kaspa-auth/source

# Pull latest changes
git pull origin main

# Build new version
cargo build --release

# Stop service
sudo systemctl stop kaspa-auth

# Update binary
sudo cp target/release/kaspa-auth /opt/kaspa-auth/

# Start service
sudo systemctl start kaspa-auth

# Verify health
sleep 5
./health-check.sh

echo "✅ Update completed"
```

---

*Self-hosting gives you complete control over your authentication infrastructure while maintaining the security benefits of blockchain-based authentication.*
```

---

## 📋 **Phase 3: Example Applications (90 minutes)**

### 3.1 React Integration Example (45 min)
```bash
mkdir -p examples/react-app
cd examples/react-app
npx create-react-app . --template typescript
npm install @kaspa/auth-sdk
```

```typescript
// examples/react-app/src/hooks/useKaspaAuth.ts
import { useState, useEffect, useCallback } from 'react';
import { KaspaAuth, generateKeypair, signMessage } from '@kaspa/auth-sdk';

interface AuthState {
  isAuthenticated: boolean;
  sessionToken: string | null;
  publicKey: string | null;
  isLoading: boolean;
  error: string | null;
}

export const useKaspaAuth = (endpoint: string = 'https://auth.kaspa.org') => {
  const [auth] = useState(() => new KaspaAuth({ endpoint }));
  const [state, setState] = useState<AuthState>({
    isAuthenticated: false,
    sessionToken: null,
    publicKey: null,
    isLoading: false,
    error: null,
  });

  // Initialize from localStorage
  useEffect(() => {
    const sessionToken = localStorage.getItem('kaspa_session_token');
    const publicKey = localStorage.getItem('kaspa_public_key');
    
    if (sessionToken && publicKey) {
      setState(prev => ({
        ...prev,
        isAuthenticated: true,
        sessionToken,
        publicKey,
      }));
    }
  }, []);

  const login = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true, error: null }));
    
    try {
      // Get or generate keypair
      let privateKey = localStorage.getItem('kaspa_private_key');
      let publicKey = localStorage.getItem('kaspa_public_key');
      
      if (!privateKey || !publicKey) {
        const keypair = generateKeypair();
        privateKey = Buffer.from(keypair.privateKey).toString('hex');
        publicKey = keypair.publicKey;
        
        localStorage.setItem('kaspa_private_key', privateKey);
        localStorage.setItem('kaspa_public_key', publicKey);
      }
      
      // Authenticate
      const { episodeId } = await auth.startAuthentication(publicKey);
      const { nonce } = await auth.requestChallenge(episodeId);
      const signature = signMessage(Buffer.from(privateKey, 'hex'), nonce);
      const { sessionToken } = await auth.verifySignature(episodeId, signature, nonce);
      
      // Store session
      localStorage.setItem('kaspa_session_token', sessionToken);
      localStorage.setItem('kaspa_episode_id', episodeId.toString());
      
      setState(prev => ({
        ...prev,
        isAuthenticated: true,
        sessionToken,
        publicKey,
        isLoading: false,
      }));
      
    } catch (error) {
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Authentication failed',
        isLoading: false,
      }));
    }
  }, [auth]);

  const logout = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true }));
    
    try {
      const sessionToken = localStorage.getItem('kaspa_session_token');
      const episodeId = localStorage.getItem('kaspa_episode_id');
      
      if (sessionToken && episodeId) {
        await auth.revokeSession(parseInt(episodeId), sessionToken);
      }
    } catch (error) {
      console.warn('Logout warning:', error);
    } finally {
      // Clear local storage
      localStorage.removeItem('kaspa_session_token');
      localStorage.removeItem('kaspa_episode_id');
      
      setState({
        isAuthenticated: false,
        sessionToken: null,
        publicKey: null,
        isLoading: false,
        error: null,
      });
    }
  }, [auth]);

  return {
    ...state,
    login,
    logout,
  };
};
```

```tsx
// examples/react-app/src/components/AuthProvider.tsx
import React, { createContext, useContext, ReactNode } from 'react';
import { useKaspaAuth } from '../hooks/useKaspaAuth';

interface AuthContextType {
  isAuthenticated: boolean;
  sessionToken: string | null;
  publicKey: string | null;
  isLoading: boolean;
  error: string | null;
  login: () => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const auth = useKaspaAuth();
  
  return (
    <AuthContext.Provider value={auth}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};
```

```tsx
// examples/react-app/src/components/LoginButton.tsx
import React from 'react';
import { useAuth } from './AuthProvider';

export const LoginButton: React.FC = () => {
  const { isAuthenticated, isLoading, error, login, logout, publicKey } = useAuth();

  if (isAuthenticated) {
    return (
      <div className="auth-success">
        <p>✅ Authenticated as {publicKey?.slice(0, 20)}...</p>
        <button onClick={logout} disabled={isLoading}>
          {isLoading ? '🔄 Logging out...' : '🚪 Logout'}
        </button>
      </div>
    );
  }

  return (
    <div className="auth-login">
      <button onClick={login} disabled={isLoading}>
        {isLoading ? '🔄 Authenticating...' : '🔐 Login with Kaspa'}
      </button>
      {error && <p className="error">❌ {error}</p>}
    </div>
  );
};
```

### 3.2 Complete React App (45 min)
```tsx
// examples/react-app/src/App.tsx
import React from 'react';
import { AuthProvider } from './components/AuthProvider';
import { LoginButton } from './components/LoginButton';
import { ProtectedContent } from './components/ProtectedContent';
import './App.css';

function App() {
  return (
    <AuthProvider>
      <div className="App">
        <header className="App-header">
          <h1>🔐 Kaspa Auth React Demo</h1>
          <p>Unphishable authentication for React applications</p>
          
          <LoginButton />
          <ProtectedContent />
        </header>
      </div>
    </AuthProvider>
  );
}

export default App;
```

```tsx
// examples/react-app/src/components/ProtectedContent.tsx
import React, { useState, useEffect } from 'react';
import { useAuth } from './AuthProvider';

export const ProtectedContent: React.FC = () => {
  const { isAuthenticated, sessionToken } = useAuth();
  const [userData, setUserData] = useState<any>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (isAuthenticated && sessionToken) {
      fetchProtectedData();
    }
  }, [isAuthenticated, sessionToken]);

  const fetchProtectedData = async () => {
    setLoading(true);
    try {
      // Example: Call your protected API
      const response = await fetch('/api/user-data', {
        headers: {
          'Authorization': `Bearer ${sessionToken}`,
        },
      });
      
      if (response.ok) {
        const data = await response.json();
        setUserData(data);
      }
    } catch (error) {
      console.error('Failed to fetch protected data:', error);
    } finally {
      setLoading(false);
    }
  };

  if (!isAuthenticated) {
    return (
      <div className="protected-content">
        <h3>🔒 Protected Content</h3>
        <p>Please authenticate to view this content.</p>
      </div>
    );
  }

  return (
    <div className="protected-content">
      <h3>✅ Protected Content</h3>
      <p>Welcome! You have access to this protected area.</p>
      
      {loading ? (
        <p>🔄 Loading user data...</p>
      ) : userData ? (
        <div className="user-data">
          <h4>Your Data:</h4>
          <pre>{JSON.stringify(userData, null, 2)}</pre>
        </div>
      ) : (
        <button onClick={fetchProtectedData}>
          📊 Fetch Protected Data
        </button>
      )}
      
      <div className="session-info">
        <h4>Session Info:</h4>
        <p><strong>Token:</strong> {sessionToken?.slice(0, 20)}...</p>
        <p><strong>Status:</strong> Active</p>
      </div>
    </div>
  );
};
```

```css
/* examples/react-app/src/App.css */
.App {
  text-align: center;
  padding: 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
}

.App-header {
  background-color: #282c34;
  padding: 40px;
  color: white;
  border-radius: 10px;
  margin: 20px auto;
  max-width: 800px;
}

.auth-login button, .auth-success button {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 16px;
  cursor: pointer;
  margin: 10px;
  transition: transform 0.2s;
}

.auth-login button:hover, .auth-success button:hover {
  transform: translateY(-2px);
}

.auth-login button:disabled, .auth-success button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.auth-success {
  background: rgba(40, 167, 69, 0.1);
  border: 2px solid #28a745;
  border-radius: 8px;
  padding: 20px;
  margin: 20px 0;
}

.protected-content {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 30px;
  margin: 30px 0;
  text-align: left;
}

.protected-content h3 {
  margin-top: 0;
  text-align: center;
}

.user-data {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
  padding: 15px;
  margin: 15px 0;
}

.user-data pre {
  background: rgba(0, 0, 0, 0.3);
  padding: 10px;
  border-radius: 4px;
  overflow-x: auto;
  font-size: 12px;
}

.session-info {
  background: rgba(255, 193, 7, 0.1);
  border: 1px solid #ffc107;
  border-radius: 4px;
  padding: 15px;
  margin: 15px 0;
  font-size: 14px;
}

.error {
  color: #dc3545;
  background: rgba(220, 53, 69, 0.1);
  border: 1px solid #dc3545;
  border-radius: 4px;
  padding: 10px;
  margin: 10px 0;
}
```

---

## 📋 **Phase 4: Deployment Documentation (60 minutes)**

### 4.1 Production Deployment Guide (30 min)
```markdown
<!-- docs/deployment.md -->
# 🚀 Production Deployment

## Deployment Options

### 1. Managed Cloud Hosting
For most applications, using a managed Kaspa Auth service is recommended:

```javascript
const auth = new KaspaAuth({
    endpoint: 'https://auth.kaspa.org'  // Managed service
});
```

**Benefits:**
- Professional SLA and support
- Automatic updates and security patches
- High availability and global CDN
- No infrastructure management

### 2. Self-Hosted Cloud
For organizations requiring full control:

#### AWS Deployment
```yaml
# aws-deployment.yml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'Kaspa Auth Coordinator on AWS'

Parameters:
  InstanceType:
    Type: String
    Default: t3.medium
    Description: EC2 instance type

Resources:
  # EC2 Instance
  KaspaAuthInstance:
    Type: AWS::EC2::Instance
    Properties:
      ImageId: ami-0abcdef1234567890  # Ubuntu 22.04 LTS
      InstanceType: !Ref InstanceType
      SecurityGroupIds:
        - !Ref KaspaAuthSecurityGroup
      IamInstanceProfile: !Ref KaspaAuthInstanceProfile
      UserData:
        Fn::Base64: !Sub |
          #!/bin/bash
          apt-get update
          apt-get install -y docker.io nginx certbot python3-certbot-nginx
          
          # Deploy Kaspa Auth
          docker run -d \
            --name kaspa-auth \
            --restart unless-stopped \
            -p 8080:8080 \
            -e KASPA_NETWORK=mainnet \
            kaspa/auth-coordinator:latest

  # Security Group
  KaspaAuthSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Security group for Kaspa Auth
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 22
          ToPort: 22
          CidrIp: 0.0.0.0/0
        - IpProtocol: tcp
          FromPort: 80
          ToPort: 80
          CidrIp: 0.0.0.0/0
        - IpProtocol: tcp
          FromPort: 443
          ToPort: 443
          CidrIp: 0.0.0.0/0

  # Application Load Balancer
  KaspaAuthALB:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Type: application
      Subnets:
        - subnet-12345678
        - subnet-87654321
      SecurityGroups:
        - !Ref KaspaAuthSecurityGroup

Outputs:
  LoadBalancerDNS:
    Description: 'DNS name of the load balancer'
    Value: !GetAtt KaspaAuthALB.DNSName
```

#### Google Cloud Platform
```yaml
# gcp-deployment.yml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kaspa-auth
spec:
  replicas: 2
  selector:
    matchLabels:
      app: kaspa-auth
  template:
    metadata:
      labels:
        app: kaspa-auth
    spec:
      containers:
      - name: kaspa-auth
        image: kaspa/auth-coordinator:latest
        ports:
        - containerPort: 8080
        env:
        - name: KASPA_NETWORK
          value: "mainnet"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: kaspa-auth-service
spec:
  selector:
    app: kaspa-auth
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### 3. On-Premises Deployment
For organizations with strict data sovereignty requirements:

#### High Availability Setup
```yaml
# ha-deployment.yml
version: '3.8'
services:
  kaspa-auth-1:
    image: kaspa/auth-coordinator:latest
    environment:
      - KASPA_NETWORK=mainnet
      - KASPA_INSTANCE_ID=coordinator-1
    volumes:
      - ./config:/app/config:ro
    networks:
      - kaspa-auth-network
      
  kaspa-auth-2:
    image: kaspa/auth-coordinator:latest
    environment:
      - KASPA_NETWORK=mainnet
      - KASPA_INSTANCE_ID=coordinator-2
    volumes:
      - ./config:/app/config:ro
    networks:
      - kaspa-auth-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx-ha.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - kaspa-auth-1
      - kaspa-auth-2
    networks:
      - kaspa-auth-network

  redis:
    image: redis:alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis-data:/data
    networks:
      - kaspa-auth-network

networks:
  kaspa-auth-network:
    driver: bridge

volumes:
  redis-data:
```

## Environment Configuration

### Production Environment Variables
```bash
# Production .env file
KASPA_NETWORK=mainnet
KASPA_KASPAD_URL=your-kaspad-node.com:16210
KASPA_PORT=8080
KASPA_TLS_ENABLED=true
KASPA_TLS_CERT_PATH=/etc/ssl/certs/kaspa-auth.crt
KASPA_TLS_KEY_PATH=/etc/ssl/private/kaspa-auth.key

# Session configuration
KASPA_SESSION_LIFETIME=3600
KASPA_MAX_AUTH_ATTEMPTS=5
KASPA_RATE_LIMIT_WINDOW=60

# Redis (for HA deployments)
REDIS_URL=redis://username:password@redis:6379/0

# Monitoring
RUST_LOG=info
KASPA_METRICS_ENABLED=true
KASPA_METRICS_PORT=9090

# Security
KASPA_CORS_ORIGIN=https://yourapp.com
KASPA_MAX_REQUEST_SIZE=1048576
```

### SSL/TLS Certificate
```bash
# Let's Encrypt (recommended)
sudo certbot --nginx -d auth.yourapp.com

# Or use your own certificates
sudo cp your-cert.crt /etc/ssl/certs/kaspa-auth.crt
sudo cp your-key.key /etc/ssl/private/kaspa-auth.key
sudo chown kaspa-auth:kaspa-auth /etc/ssl/private/kaspa-auth.key
sudo chmod 600 /etc/ssl/private/kaspa-auth.key
```

## Monitoring and Observability

### Health Check Endpoints
```bash
# Basic health
curl https://auth.yourapp.com/health

# Detailed status
curl https://auth.yourapp.com/status

# Metrics (Prometheus format)
curl https://auth.yourapp.com:9090/metrics
```

### Prometheus Configuration
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'kaspa-auth'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 30s
    metrics_path: /metrics
```

### Grafana Dashboard
```json
{
  "dashboard": {
    "title": "Kaspa Auth Monitoring",
    "panels": [
      {
        "title": "Authentication Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(kaspa_auth_requests_total[5m])",
            "legendFormat": "Auth Requests/sec"
          }
        ]
      },
      {
        "title": "Session Token Issuance",
        "type": "graph", 
        "targets": [
          {
            "expr": "kaspa_auth_sessions_active",
            "legendFormat": "Active Sessions"
          }
        ]
      }
    ]
  }
}
```

## Performance Optimization

### Caching Strategy
```rust
// config/production.toml
[cache]
enabled = true
challenge_ttl = 300      # 5 minutes
session_cache_size = 10000
redis_url = "redis://localhost:6379/0"

[rate_limiting]
requests_per_minute = 60
burst_size = 20
cleanup_interval = 3600
```

### Database Optimization
```sql
-- PostgreSQL indices for session storage (if using DB)
CREATE INDEX idx_sessions_token ON sessions(session_token);
CREATE INDEX idx_sessions_expiry ON sessions(expires_at);
CREATE INDEX idx_sessions_pubkey ON sessions(public_key);

-- Clean up expired sessions
DELETE FROM sessions WHERE expires_at < NOW();
```

### CDN Configuration
```javascript
// CloudFlare example
const cfConfig = {
  // Cache static assets
  "rules": [
    {
      "expression": "http.request.uri.path matches \"^/static/.*\"",
      "action": "cache",
      "cache_ttl": 86400
    },
    {
      "expression": "http.request.uri.path matches \"^/auth/.*\"",
      "action": "bypass_cache"
    }
  ]
};
```

## Security Hardening

### Firewall Rules
```bash
# Ubuntu UFW
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# Fail2Ban
sudo apt install fail2ban
# Configure /etc/fail2ban/jail.local with kaspa-auth rules
```

### Security Headers
```nginx
# nginx security headers
add_header X-Frame-Options DENY always;
add_header X-Content-Type-Options nosniff always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy strict-origin-when-cross-origin always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';" always;
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
```

### Audit Logging
```toml
# config/production.toml
[audit]
enabled = true
log_file = "/var/log/kaspa-auth/audit.log"
log_format = "json"
include_ip = true
include_user_agent = true

[events]
log_auth_attempts = true
log_session_creation = true
log_session_revocation = true
log_failures = true
```

---

*Production deployment requires careful planning and ongoing maintenance. Consider using the managed service for most applications.*
```

### 4.2 Create Contributing Guide (30 min)
```markdown
<!-- CONTRIBUTING.md -->
# 🤝 Contributing to Kaspa Auth

Thank you for your interest in contributing to Kaspa Auth! This guide will help you get started.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

## Getting Started

### Prerequisites
- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs)
- **Node.js 18+**: For SDK development
- **Git**: For version control
- **Kaspa Node**: Access to testnet-10 for testing

### Development Setup
```bash
# Clone the repository
git clone https://github.com/kaspa/kaspa-auth
cd kaspa-auth

# Build the project
cargo build

# Run tests
cargo test

# Install pre-commit hooks
cargo install cargo-fmt cargo-clippy
echo 'cargo fmt --all -- --check && cargo clippy -- -D warnings' > .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Project Structure

```
kaspa-auth/
├── src/
│   ├── core/              # Authentication episode and commands
│   ├── api/               # HTTP and WebSocket coordination  
│   ├── cli/               # Command-line interface
│   ├── crypto/            # Cryptographic utilities
│   └── wallet/            # Wallet management
├── sdk/
│   ├── auth-sdk/          # REST API client (TypeScript)
│   └── p2p-sdk/           # WebSocket client (TypeScript)
├── examples/
│   ├── pure-p2p/          # WebSocket-first demo
│   ├── react-app/         # React integration
│   └── kaspa-poker/       # Poker tournament example
├── docs/                  # Documentation
└── tests/                 # Integration tests
```

## How to Contribute

### 1. Report Issues
- Use GitHub Issues for bug reports and feature requests
- Search existing issues before creating new ones
- Provide clear reproduction steps for bugs
- Include relevant system information

### 2. Suggest Features
- Discuss major features in GitHub Discussions first
- Consider backward compatibility
- Provide use cases and rationale
- Include implementation suggestions when possible

### 3. Submit Pull Requests

#### Before You Start
- Check existing issues and PRs
- Discuss large changes in an issue first
- Follow the coding standards below

#### PR Process
1. **Fork and Branch**
   ```bash
   git fork https://github.com/kaspa/kaspa-auth
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clear, focused commits
   - Add tests for new functionality
   - Update documentation as needed
   - Follow coding standards

3. **Test Your Changes**
   ```bash
   # Run all tests
   cargo test
   
   # Test specific component
   cargo test auth_episode
   
   # Integration tests
   cargo test --test integration
   
   # SDK tests
   cd sdk/auth-sdk && npm test
   cd sdk/p2p-sdk && npm test
   ```

4. **Submit PR**
   - Write clear PR description
   - Reference related issues
   - Include testing instructions
   - Update CHANGELOG.md

## Coding Standards

### Rust Code Style
```rust
// ✅ Good: Clear function names and documentation
/// Verifies a signature against a public key and message
pub fn verify_signature(
    public_key: &PubKey, 
    message: &str, 
    signature: &str
) -> Result<bool, Error> {
    // Implementation
}

// ❌ Bad: Unclear names and no documentation
pub fn verify(pk: &PubKey, msg: &str, sig: &str) -> bool {
    // Implementation
}
```

### Error Handling
```rust
// ✅ Good: Specific error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Session expired at {expires_at}")]
    SessionExpired { expires_at: SystemTime },
}

// ❌ Bad: Generic errors
return Err("something went wrong".into());
```

### Security Guidelines
```rust
// ✅ Good: No secrets in logs
match verify_signature(&pubkey, &challenge, &signature) {
    Ok(true) => Ok("Authentication successful"),
    _ => Err("Authentication failed"), // Don't leak signature details
}

// ❌ Bad: Information leakage
match verify_signature(&pubkey, &challenge, &signature) {
    Err(e) => Err(format!("Signature verification failed: {}", e)),
}
```

### TypeScript Style
```typescript
// ✅ Good: Proper types and error handling
export class KaspaAuth {
    async startAuthentication(publicKey: string): Promise<{ episodeId: number }> {
        if (!publicKey || !publicKey.startsWith('kaspa')) {
            throw new Error('Invalid Kaspa public key format');
        }
        
        // Implementation
    }
}

// ❌ Bad: Any types and poor error handling
export class KaspaAuth {
    async startAuthentication(publicKey: any): Promise<any> {
        // Implementation that might throw
    }
}
```

## Testing Guidelines

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signature_verification() {
        let (private_key, public_key) = generate_test_keypair();
        let message = "test_challenge_123";
        let signature = sign_message(&private_key, message);
        
        assert!(verify_signature(&public_key, message, &signature).unwrap());
    }
    
    #[test]
    fn test_invalid_signature_rejected() {
        let (_, public_key) = generate_test_keypair();
        let message = "test_challenge_123";
        let invalid_signature = "invalid_signature";
        
        assert!(!verify_signature(&public_key, message, invalid_signature).unwrap_or(false));
    }
}
```

### Integration Tests
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_full_authentication_flow() {
    // Start test coordinator
    let coordinator = start_test_coordinator().await;
    
    // Test complete flow
    let client = TestClient::new(coordinator.endpoint());
    let result = client.authenticate().await;
    
    assert!(result.is_ok());
    assert!(result.unwrap().session_token.len() > 0);
    
    coordinator.shutdown().await;
}
```

## Documentation

### Code Documentation
- All public functions must have rustdoc comments
- Include examples for complex functions
- Document error conditions
- Use `cargo doc --open` to preview documentation

### User Documentation
- Update relevant docs in `docs/` directory
- Include code examples that work
- Test all examples before submitting
- Consider different user skill levels

## Release Process

### Versioning
We follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist
- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml and package.json
- [ ] Security review for changes
- [ ] Performance impact assessed

## Development Workflow

### Feature Development
```bash
# 1. Create feature branch
git checkout -b feature/session-management

# 2. Develop and test
cargo test
npm test

# 3. Commit with clear messages
git commit -m "feat: add session refresh endpoint

- Implements automatic session renewal
- Adds refresh token rotation
- Updates documentation and tests"

# 4. Push and create PR
git push origin feature/session-management
```

### Commit Message Format
```
type(scope): short description

Longer description if needed

- Bullet points for details
- Reference issues with #123
- Breaking changes noted with BREAKING:
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

## Community

### Getting Help
- **GitHub Discussions**: General questions and ideas
- **GitHub Issues**: Bug reports and feature requests  
- **Discord**: Real-time community chat
- **Email**: security@kaspa.org for security issues

### Recognition
Contributors are recognized in:
- CONTRIBUTORS.md file
- Release notes for significant contributions
- Annual contributor spotlights

## Security

### Reporting Security Issues
- **DO NOT** open public issues for security vulnerabilities
- Email security@kaspa.org with details
- Include steps to reproduce
- We'll respond within 48 hours

### Security Review Process
- All crypto changes require security review
- Authentication logic changes need approval from maintainers
- Use `cargo audit` to check for vulnerable dependencies

---

Thank you for contributing to Kaspa Auth! Your efforts help make unphishable authentication accessible to everyone. 🚀
```

<function_calls>
<invoke name="TodoWrite">
<parameter name="todos">[{"id": "create_multi_session_roadmap", "content": "Create detailed multi-session roadmap with Session_2.md, Session_3.md, etc.", "status": "completed", "priority": "high"}]