# 🚨 CRITICAL: RUN ALL COMMANDS FROM THIS DIRECTORY!

## ⚠️ You MUST be in `examples/kaspa-auth/` directory to run kaspa-auth commands!

### ✅ Verify you're in the right place:
```bash
pwd
# Should show: .../kdapp/examples/kaspa-auth
```

### ✅ Working commands (from THIS directory):
```bash
cargo run --bin kaspa-auth -- wallet-status
cargo run --bin kaspa-auth -- http-peer --port 8080  
cargo run --bin kaspa-auth -- authenticate
cargo run --bin kaspa-auth -- revoke-session --episode-id 123 --session-token sess_xyz
```

### ❌ If you run from kdapp root, you'll get:
```
error: no bin target named `kaspa-auth`
```

### 🔧 Quick fix if in wrong directory:
```bash
cd examples/kaspa-auth/  # From kdapp root
# OR
cd /full/path/to/kdapp/examples/kaspa-auth/  # From anywhere
```

---

**This file exists because EVERYONE makes this mistake!** 😅

Save yourself hours of debugging - always check your directory first!