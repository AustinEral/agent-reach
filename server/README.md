# agent-reach-server

DID-based discovery registry server for AI agents.

## Usage

```bash
# Default port (3001)
cargo run -p agent-reach-server

# Custom port
cargo run -p agent-reach-server -- --port 8080
```

## API

### Handshake (Authentication)

#### POST /hello

Start a handshake. Returns a challenge.

```bash
curl -X POST http://localhost:3001/hello \
  -H "Content-Type: application/json" \
  -d '{"type":"Hello","version":"1.0","did":"did:key:z6Mk...","protocols":["aip/1.0"],"timestamp":1234567890}'
```

#### POST /proof

Complete handshake with proof. Returns session ID.

```bash
curl -X POST http://localhost:3001/proof \
  -H "Content-Type: application/json" \
  -d '{"type":"Proof",...}'
```

### Registration (Requires Session)

#### POST /register

Register your endpoint.

```bash
curl -X POST http://localhost:3001/register \
  -H "Authorization: Bearer <session_id>" \
  -H "Content-Type: application/json" \
  -d '{"endpoint":"wss://my-agent:8080","ttl":3600}'
```

Response:
```json
{"ok":true,"did":"did:key:z6Mk...","expires_at":1234567890}
```

#### POST /deregister

Remove your registration.

```bash
curl -X POST http://localhost:3001/deregister \
  -H "Authorization: Bearer <session_id>"
```

### Lookup (Public)

#### GET /lookup/:did

Look up an agent's endpoint.

```bash
curl http://localhost:3001/lookup/did:key:z6Mk...
```

Response:
```json
{
  "did": "did:key:z6Mk...",
  "endpoint": "wss://my-agent:8080",
  "status": "online",
  "registered_at": 1234567890,
  "expires_at": 1234571490
}
```

### Health

#### GET /health

Returns `ok` if server is running.

## Configuration

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--port` | - | 3001 | Port to listen on |

## Security

- All registrations require authentication via agent-id handshake
- Sessions expire after 5 minutes
- Registrations expire based on TTL (default: 1 hour)
