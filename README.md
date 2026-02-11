# agent-reach

DID-based discovery for AI agents. The phonebook for the agentic web.

## What It Does

Agents have identities (DIDs via [agent-id](https://github.com/AustinEral/agent-id)), but how do they find each other? agent-reach is a simple registry:

1. **Register**: "I'm `did:key:z6Mk...`, reach me at `wss://...`"
2. **Lookup**: "Where is `did:key:z6Mk...`?" → `wss://...`

That's it. How agents actually communicate is up to them — A2A, ACP, raw WebSockets, whatever works.

## Quick Start

```bash
# Clone
git clone https://github.com/AustinEral/agent-reach.git
cd agent-reach

# Start the server
cargo run -p agent-reach-server

# In another terminal, create an identity (if you haven't)
cd ../agent-id
cargo run --bin agent-id -- identity generate

# Authenticate with the registry
cd ../agent-reach
export SESSION=$(cargo run -p agent-reach-cli -- auth http://localhost:3001)

# Register your endpoint
cargo run -p agent-reach-cli -- register http://localhost:3001 -e wss://my-agent:8080

# Look up an agent
cargo run -p agent-reach-cli -- lookup http://localhost:3001 did:key:z6Mk...
```

## Architecture

```
agent-reach/
  server/     # Registry server (agent-reach-server)
  cli/        # CLI client (agent-reach)
```

### Server

The registry server stores DID → endpoint mappings with TTL-based expiration.

```bash
# Start with default port (3001)
cargo run -p agent-reach-server

# Custom port
cargo run -p agent-reach-server -- --port 8080
```

**Endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| POST | `/hello` | Start handshake |
| POST | `/proof` | Complete handshake, get session |
| POST | `/register` | Register endpoint (requires session) |
| POST | `/deregister` | Remove registration (requires session) |
| GET | `/lookup/:did` | Look up agent endpoint |
| GET | `/health` | Health check |

### CLI

The CLI client interacts with any agent-reach server.

```bash
# Authenticate (outputs session ID to stdout)
cargo run -p agent-reach-cli -- auth <server>

# Register endpoint (requires SESSION env var or --session)
export SESSION=$(cargo run -p agent-reach-cli -- auth http://localhost:3001)
cargo run -p agent-reach-cli -- register <server> -e <endpoint> [-t <ttl>]

# Look up agent (outputs endpoint to stdout)
cargo run -p agent-reach-cli -- lookup <server> <did>

# Deregister
cargo run -p agent-reach-cli -- deregister <server>
```

## Authentication Flow

agent-reach uses the [agent-id handshake protocol](https://github.com/AustinEral/agent-id) for authentication:

```
Agent                              Registry
  │                                    │
  │  POST /hello { did }               │
  │───────────────────────────────────▶│
  │         Challenge                  │
  │◀───────────────────────────────────│
  │                                    │
  │  POST /proof { proof }             │
  │───────────────────────────────────▶│
  │         ProofAccepted { session }  │
  │◀───────────────────────────────────│
  │                                    │
  │  POST /register { endpoint }       │
  │  Authorization: Bearer <session>   │
  │───────────────────────────────────▶│
  │         { ok, did, expires_at }    │
  │◀───────────────────────────────────│
```

This proves the agent controls the DID's private key before allowing registration.

## How It Fits

```
┌─────────────────────────────────┐
│     A2A / ACP / custom          │  ← communication protocol
├─────────────────────────────────┤
│     Transport (WebSocket,       │  ← how messages flow
│     HTTP, Nostr, local socket)  │
├─────────────────────────────────┤
│         agent-reach             │  ← where to find agents
├─────────────────────────────────┤
│          agent-id               │  ← who agents are
└─────────────────────────────────┘
```

## Related Projects

- [agent-id](https://github.com/AustinEral/agent-id) — Cryptographic identity for agents
- [agent-id-mcp](https://github.com/AustinEral/agent-id-mcp) — MCP server for agent-id

## Documentation

- [GOALS.md](docs/GOALS.md) — What we're building
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) — How it works
- [HANDSHAKE.md](docs/HANDSHAKE.md) — Security model

## License

Apache-2.0
