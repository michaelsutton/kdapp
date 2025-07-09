# Kaspa Auth - Web UI

This folder contains the static web assets for the Kaspa Auth dashboard.

## Structure

```
public/
├── index.html          # Main dashboard (served at / and /web)
├── css/               # CSS stylesheets (future)
├── js/                # JavaScript modules (future)
└── assets/            # Images, icons, etc. (future)
```

## Features

- **Real-time Authentication Flow**: Complete test of the auth system
- **WebSocket Integration**: Live updates for challenge issuance and auth events
- **Responsive Design**: Works on desktop and mobile
- **Auto-generated Keypairs**: No need to provide your own keys for testing

## Usage

1. Start the server:
   ```bash
   cargo run -- http-peer --port 8080
   ```

2. Open your browser to:
   - http://localhost:8080 (main dashboard)
   - http://localhost:8080/web (alternative URL)

3. Click "Start Authentication Flow" to test the complete system

## Development

The Web UI is embedded into the Rust binary using `include_str!()` for easy deployment. To modify:

1. Edit `public/index.html`
2. Rebuild with `cargo build`
3. The changes will be included in the next server startup

## API Integration

The dashboard uses these endpoints:
- `POST /auth/start` - Create episode
- `POST /auth/request-challenge` - Request challenge
- `POST /auth/verify` - Submit verification
- `GET /auth/status/{id}` - Check episode status
- `WebSocket /ws` - Real-time updates