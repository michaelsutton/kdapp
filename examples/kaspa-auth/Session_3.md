# 📦 Session 3: Create NPM SDK Packages (Market Ready)

**Prerequisites**: Session 2 completed (pure P2P example working)

## 🎯 **Session Goal: Package for Distribution**

**Why This**: Transform your working authentication system into distributable NPM packages that developers can actually use. This creates the "Login with Kaspa" SDK that traditional web developers expect.

**Time Estimate**: 4-5 hours  
**Outcome**: Published NPM packages `@kaspa/auth-sdk` and `@kaspa/p2p-sdk` ready for developers

---

## 📋 **Phase 1: Create SDK Package Structure (60 minutes)**

### 1.1 Set Up NPM Package Directories (20 min)
```bash
mkdir -p sdk/auth-sdk/src
mkdir -p sdk/auth-sdk/dist
mkdir -p sdk/p2p-sdk/src
mkdir -p sdk/p2p-sdk/dist

# Package.json files
touch sdk/auth-sdk/package.json
touch sdk/auth-sdk/tsconfig.json
touch sdk/auth-sdk/src/index.ts
touch sdk/p2p-sdk/package.json
touch sdk/p2p-sdk/tsconfig.json
touch sdk/p2p-sdk/src/index.ts
```

### 1.2 Configure Auth SDK Package (20 min)
```json
// sdk/auth-sdk/package.json
{
  "name": "@kaspa/auth-sdk",
  "version": "0.1.0",
  "description": "Easy authentication for Kaspa blockchain - REST API client",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "prepublishOnly": "npm run build"
  },
  "keywords": ["kaspa", "blockchain", "authentication", "web3", "crypto"],
  "author": "Kaspa Community",
  "license": "MIT",
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0",
    "jest": "^29.0.0"
  },
  "dependencies": {
    "secp256k1": "^5.0.0"
  }
}
```

### 1.3 Configure P2P SDK Package (20 min)
```json
// sdk/p2p-sdk/package.json
{
  "name": "@kaspa/p2p-sdk",
  "version": "0.1.0", 
  "description": "Pure P2P authentication for Kaspa blockchain - WebSocket client",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "prepublishOnly": "npm run build"
  },
  "keywords": ["kaspa", "blockchain", "p2p", "websocket", "decentralized"],
  "author": "Kaspa Community",
  "license": "MIT",
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0", 
    "@types/ws": "^8.5.0",
    "jest": "^29.0.0"
  },
  "dependencies": {
    "ws": "^8.14.0",
    "secp256k1": "^5.0.0"
  }
}
```

---

## 📋 **Phase 2: Build REST API SDK (90 minutes)**

### 2.1 Create KaspaAuth Class (45 min)
```typescript
// sdk/auth-sdk/src/index.ts
export interface AuthConfig {
  endpoint: string;
  network?: 'mainnet' | 'testnet-10' | 'testnet-11';
  timeout?: number;
}

export interface AuthResult {
  episodeId: number;
  sessionToken: string;
  expiresAt: Date;
}

export class KaspaAuth {
  private config: Required<AuthConfig>;
  
  constructor(config: AuthConfig) {
    this.config = {
      endpoint: config.endpoint,
      network: config.network || 'testnet-10',
      timeout: config.timeout || 30000
    };
  }
  
  /**
   * Start authentication process
   * @param publicKey Kaspa public key in bech32 format
   * @returns Episode ID for tracking authentication
   */
  async startAuthentication(publicKey: string): Promise<{ episodeId: number }> {
    const response = await fetch(`${this.config.endpoint}/auth/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        public_key: publicKey,
        network: this.config.network 
      })
    });
    
    if (!response.ok) {
      throw new Error(`Authentication start failed: ${response.statusText}`);
    }
    
    return await response.json();
  }
  
  /**
   * Request authentication challenge
   * @param episodeId Episode ID from startAuthentication
   * @returns Challenge nonce to sign
   */
  async requestChallenge(episodeId: number): Promise<{ nonce: string }> {
    const response = await fetch(`${this.config.endpoint}/auth/challenge/${episodeId}`, {
      method: 'GET'
    });
    
    if (!response.ok) {
      throw new Error(`Challenge request failed: ${response.statusText}`);
    }
    
    return await response.json();
  }
  
  /**
   * Verify signed challenge and complete authentication
   * @param episodeId Episode ID
   * @param signature Signed challenge nonce
   * @param nonce Original challenge nonce
   * @returns Session token and expiry
   */
  async verifySignature(
    episodeId: number, 
    signature: string, 
    nonce: string
  ): Promise<AuthResult> {
    const response = await fetch(`${this.config.endpoint}/auth/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        episode_id: episodeId,
        signature,
        nonce
      })
    });
    
    if (!response.ok) {
      throw new Error(`Signature verification failed: ${response.statusText}`);
    }
    
    const result = await response.json();
    return {
      episodeId,
      sessionToken: result.session_token,
      expiresAt: new Date(result.expires_at)
    };
  }
  
  /**
   * Revoke active session (logout)
   * @param episodeId Episode ID
   * @param sessionToken Active session token
   * @returns Transaction ID of revocation
   */
  async revokeSession(episodeId: number, sessionToken: string): Promise<{ txId: string }> {
    const response = await fetch(`${this.config.endpoint}/auth/revoke-session`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        episode_id: episodeId,
        session_token: sessionToken
      })
    });
    
    if (!response.ok) {
      throw new Error(`Session revocation failed: ${response.statusText}`);
    }
    
    return await response.json();
  }
}

// Export helper functions
export { generateKeypair, signMessage, verifySignature } from './crypto';
export type { AuthConfig, AuthResult };
```

### 2.2 Add Crypto Helper Functions (30 min)
```typescript
// sdk/auth-sdk/src/crypto.ts
import * as secp256k1 from 'secp256k1';
import { randomBytes, createHash } from 'crypto';

export interface Keypair {
  privateKey: Uint8Array;
  publicKey: string; // bech32 format
}

export function generateKeypair(): Keypair {
  let privateKey: Uint8Array;
  do {
    privateKey = randomBytes(32);
  } while (!secp256k1.privateKeyVerify(privateKey));
  
  const publicKeyBytes = secp256k1.publicKeyCreate(privateKey, false);
  const publicKey = encodeKaspaAddress(publicKeyBytes);
  
  return { privateKey, publicKey };
}

export function signMessage(privateKey: Uint8Array, message: string): string {
  const msgHash = createHash('sha256').update(message).digest();
  const signature = secp256k1.ecdsaSign(msgHash, privateKey);
  return Buffer.from(signature.signature).toString('hex');
}

export function verifySignature(publicKey: string, message: string, signature: string): boolean {
  try {
    const msgHash = createHash('sha256').update(message).digest();
    const sigBytes = Buffer.from(signature, 'hex');
    const pubKeyBytes = decodeKaspaAddress(publicKey);
    
    return secp256k1.ecdsaVerify(sigBytes, msgHash, pubKeyBytes);
  } catch {
    return false;
  }
}

function encodeKaspaAddress(publicKey: Uint8Array): string {
  // Simplified - in production use proper Kaspa address encoding
  return `kaspatest:${Buffer.from(publicKey).toString('hex').slice(0, 40)}`;
}

function decodeKaspaAddress(address: string): Uint8Array {
  // Simplified - in production use proper Kaspa address decoding  
  const hex = address.split(':')[1];
  return Buffer.from(hex, 'hex');
}
```

### 2.3 Add TypeScript Configuration (15 min)
```json
// sdk/auth-sdk/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "**/*.test.ts"]
}
```

---

## 📋 **Phase 3: Build WebSocket P2P SDK (75 minutes)**

### 3.1 Create KaspaP2P Class (45 min)
```typescript
// sdk/p2p-sdk/src/index.ts
import WebSocket from 'ws';
import { EventEmitter } from 'events';

export interface P2PConfig {
  wsEndpoint: string;
  reconnectInterval?: number;
  timeout?: number;
}

export interface P2PEvents {
  'connected': () => void;
  'disconnected': () => void;
  'challenge_ready': (data: { episodeId: number; nonce: string }) => void;
  'auth_success': (data: { episodeId: number; sessionToken: string }) => void;
  'auth_failed': (data: { episodeId: number; reason: string }) => void;
  'session_revoked': (data: { episodeId: number; txId: string }) => void;
}

export class KaspaP2P extends EventEmitter {
  private ws: WebSocket | null = null;
  private config: Required<P2PConfig>;
  private reconnectTimer: NodeJS.Timeout | null = null;
  
  constructor(config: P2PConfig) {
    super();
    this.config = {
      wsEndpoint: config.wsEndpoint,
      reconnectInterval: config.reconnectInterval || 5000,
      timeout: config.timeout || 30000
    };
  }
  
  /**
   * Connect to P2P authentication network
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.config.wsEndpoint);
      
      this.ws.onopen = () => {
        console.log('🟢 Connected to Kaspa P2P network');
        this.emit('connected');
        resolve();
      };
      
      this.ws.onclose = () => {
        console.log('🔴 Disconnected from Kaspa P2P network');
        this.emit('disconnected');
        this.scheduleReconnect();
      };
      
      this.ws.onerror = (error) => {
        console.error('❌ WebSocket error:', error);
        reject(error);
      };
      
      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data.toString());
          this.handleMessage(message);
        } catch (error) {
          console.error('Failed to parse message:', error);
        }
      };
      
      // Timeout if connection takes too long
      setTimeout(() => {
        if (this.ws?.readyState !== WebSocket.OPEN) {
          reject(new Error('Connection timeout'));
        }
      }, this.config.timeout);
    });
  }
  
  /**
   * Request authentication with public key
   * @param publicKey Kaspa public key in bech32 format
   * @returns Episode ID for tracking
   */
  async requestAuth(publicKey: string): Promise<number> {
    if (!this.isConnected()) {
      throw new Error('Not connected to P2P network');
    }
    
    const episodeId = Date.now(); // Simplified ID generation
    
    this.send({
      type: 'request_auth',
      public_key: publicKey,
      episode_id: episodeId
    });
    
    return episodeId;
  }
  
  /**
   * Submit signed challenge response
   * @param episodeId Episode ID from requestAuth
   * @param signature Signed challenge nonce
   * @param nonce Original challenge nonce
   */
  async submitSignature(episodeId: number, signature: string, nonce: string): Promise<void> {
    if (!this.isConnected()) {
      throw new Error('Not connected to P2P network');
    }
    
    this.send({
      type: 'submit_signature',
      episode_id: episodeId,
      signature,
      nonce
    });
  }
  
  /**
   * Revoke active session
   * @param episodeId Episode ID
   * @param sessionToken Session token to revoke
   */
  async revokeSession(episodeId: number, sessionToken: string): Promise<void> {
    if (!this.isConnected()) {
      throw new Error('Not connected to P2P network');
    }
    
    this.send({
      type: 'revoke_session',
      episode_id: episodeId,
      session_token: sessionToken
    });
  }
  
  /**
   * Disconnect from P2P network
   */
  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
  
  private isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
  
  private send(message: any): void {
    if (this.isConnected() && this.ws) {
      this.ws.send(JSON.stringify(message));
    }
  }
  
  private handleMessage(message: any): void {
    switch (message.type) {
      case 'challenge_ready':
        this.emit('challenge_ready', {
          episodeId: message.episode_id,
          nonce: message.nonce
        });
        break;
        
      case 'auth_success':
        this.emit('auth_success', {
          episodeId: message.episode_id,
          sessionToken: message.session_token
        });
        break;
        
      case 'auth_failed':
        this.emit('auth_failed', {
          episodeId: message.episode_id,
          reason: message.reason
        });
        break;
        
      case 'session_revoked':
        this.emit('session_revoked', {
          episodeId: message.episode_id,
          txId: message.tx_id
        });
        break;
        
      default:
        console.warn('Unknown message type:', message.type);
    }
  }
  
  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;
    
    this.reconnectTimer = setTimeout(() => {
      console.log('🔄 Attempting to reconnect...');
      this.connect().catch(console.error);
      this.reconnectTimer = null;
    }, this.config.reconnectInterval);
  }
}

// Re-export crypto utilities from auth-sdk
export { generateKeypair, signMessage, verifySignature } from '@kaspa/auth-sdk';
export type { P2PConfig, P2PEvents };
```

### 3.2 Add P2P TypeScript Configuration (15 min)
```json
// sdk/p2p-sdk/tsconfig.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src", 
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "**/*.test.ts"]
}
```

### 3.3 Create Usage Examples (15 min)
```typescript
// sdk/p2p-sdk/examples/basic-usage.ts
import { KaspaP2P, generateKeypair, signMessage } from '@kaspa/p2p-sdk';

async function demonstrateP2PAuth() {
  // 1. Connect to P2P network
  const p2p = new KaspaP2P({
    wsEndpoint: 'ws://localhost:8080'
  });
  
  await p2p.connect();
  
  // 2. Generate keypair
  const { privateKey, publicKey } = generateKeypair();
  console.log('Generated public key:', publicKey);
  
  // 3. Request authentication
  const episodeId = await p2p.requestAuth(publicKey);
  console.log('Started authentication, episode:', episodeId);
  
  // 4. Listen for challenge
  p2p.on('challenge_ready', ({ episodeId, nonce }) => {
    console.log('Received challenge:', nonce);
    
    // Sign the challenge
    const signature = signMessage(privateKey, nonce);
    
    // Submit signature
    p2p.submitSignature(episodeId, signature, nonce);
  });
  
  // 5. Handle authentication result
  p2p.on('auth_success', ({ sessionToken }) => {
    console.log('✅ Authentication successful!');
    console.log('Session token:', sessionToken);
    
    // Later: revoke session
    setTimeout(() => {
      p2p.revokeSession(episodeId, sessionToken);
    }, 10000);
  });
  
  p2p.on('auth_failed', ({ reason }) => {
    console.error('❌ Authentication failed:', reason);
  });
}

demonstrateP2PAuth().catch(console.error);
```

---

## 📋 **Phase 4: Build and Test Packages (45 minutes)**

### 4.1 Build Both Packages (20 min)
```bash
# Build auth SDK
cd sdk/auth-sdk
npm install
npm run build

# Build P2P SDK  
cd ../p2p-sdk
npm install
npm run build

# Verify dist folders contain compiled JS and type definitions
ls -la dist/
```

### 4.2 Create Integration Tests (25 min)
```typescript
// sdk/auth-sdk/src/auth.test.ts
import { KaspaAuth } from './index';

describe('KaspaAuth SDK', () => {
  let auth: KaspaAuth;
  
  beforeEach(() => {
    auth = new KaspaAuth({
      endpoint: 'http://localhost:8080'
    });
  });
  
  test('should start authentication', async () => {
    // Mock fetch for testing
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ episode_id: 12345 })
    });
    
    const result = await auth.startAuthentication('kaspatest:abc123');
    expect(result.episodeId).toBe(12345);
  });
  
  test('should request challenge', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ nonce: 'challenge_123' })
    });
    
    const result = await auth.requestChallenge(12345);
    expect(result.nonce).toBe('challenge_123');
  });
});
```

---

## 📋 **Phase 5: Documentation and Publishing (30 minutes)**

### 5.1 Create README Files (20 min)
```markdown
# @kaspa/auth-sdk

Easy authentication for Kaspa blockchain applications.

## Quick Start

```bash
npm install @kaspa/auth-sdk
```

```javascript
import { KaspaAuth, generateKeypair, signMessage } from '@kaspa/auth-sdk';

// Using managed service
const auth = new KaspaAuth({
  endpoint: 'https://auth.kaspa.org'
});

// Generate keypair
const { privateKey, publicKey } = generateKeypair();

// Authenticate
const { episodeId } = await auth.startAuthentication(publicKey);
const { nonce } = await auth.requestChallenge(episodeId);
const signature = signMessage(privateKey, nonce);
const { sessionToken } = await auth.verifySignature(episodeId, signature, nonce);

console.log('Authenticated! Session:', sessionToken);
```

## Self-Hosted

```bash
# Run your own coordinator
docker run -p 8080:8080 kaspa/auth-coordinator

# Point SDK to your instance
const auth = new KaspaAuth({
  endpoint: 'http://localhost:8080'
});
```

## API Reference

[Complete API documentation here]
```

### 5.2 Prepare for Publishing (10 min)
```bash
# Test packages locally
npm pack

# Check package contents
tar -tzf kaspa-auth-sdk-0.1.0.tgz

# Ready for: npm publish
# (Don't publish yet - test thoroughly first)
```

---

## 🎉 **Success Criteria**

You'll know Session 3 is complete when:

1. **Two NPM packages built**: `@kaspa/auth-sdk` and `@kaspa/p2p-sdk`
2. **TypeScript definitions**: Full type safety for developers
3. **Working examples**: Basic usage demonstrable
4. **Tests pass**: Core functionality verified
5. **Documentation complete**: README with usage examples

---

## 💭 **Why This Session is Game-Changing**

1. **Market Ready**: Developers can `npm install` and use immediately
2. **Two Markets**: REST API for web devs, WebSocket for P2P devs  
3. **Professional**: Proper TypeScript, testing, documentation
4. **Scalable**: Template for other Kaspa tools

**Outcome**: Transform from "interesting demo" to "production-ready authentication SDK"! 📦