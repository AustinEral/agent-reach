# agent-reach-cli

CLI client for interacting with agent-reach registry servers.

## Prerequisites

You need an identity from [agent-id](https://github.com/AustinEral/agent-id):

```bash
cd agent-id
cargo run --bin agent-id -- identity generate
```

This creates `~/.config/agent-id/identity.json`.

## Usage

### Authenticate

Get a session ID by completing a handshake:

```bash
cargo run -p agent-reach-cli -- auth http://localhost:3001
```

Output (session ID to stdout):
```
019c4bf5-bd75-7be1-9c92-2224cd8fc319
```

Save it for subsequent commands:
```bash
export SESSION=$(cargo run -p agent-reach-cli -- auth http://localhost:3001)
```

### Register

Register your endpoint:

```bash
cargo run -p agent-reach-cli -- register http://localhost:3001 \
  --endpoint wss://my-agent:8080 \
  --ttl 3600
```

The `--session` flag or `SESSION` env var is required.

### Lookup

Find an agent by DID:

```bash
cargo run -p agent-reach-cli -- lookup http://localhost:3001 did:key:z6Mk...
```

Output (endpoint to stdout):
```
wss://my-agent:8080
```

### Deregister

Remove your registration:

```bash
cargo run -p agent-reach-cli -- deregister http://localhost:3001
```

## Options

### Global

| Flag | Description |
|------|-------------|
| `--identity <path>` | Path to identity file (default: `~/.config/agent-id/identity.json`) |

### auth

```bash
agent-reach auth <server> [--identity <path>]
```

### register

```bash
agent-reach register <server> --endpoint <url> [--ttl <seconds>] [--session <id>]
```

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `-e, --endpoint` | - | required | Endpoint URL |
| `-t, --ttl` | - | 3600 | Time-to-live in seconds |
| `-s, --session` | `SESSION` | required | Session ID from auth |

### lookup

```bash
agent-reach lookup <server> <did>
```

### deregister

```bash
agent-reach deregister <server> [--session <id>]
```

## Examples

### One-liner registration

```bash
SESSION=$(cargo run -p agent-reach-cli -- auth http://localhost:3001 2>/dev/null) \
  cargo run -p agent-reach-cli -- register http://localhost:3001 -e wss://my-agent:8080
```

### Script

```bash
#!/bin/bash
SERVER="http://localhost:3001"
ENDPOINT="wss://my-agent:8080"

# Authenticate
SESSION=$(agent-reach auth $SERVER 2>/dev/null)

# Register
agent-reach register $SERVER -e $ENDPOINT -s $SESSION

# Verify
agent-reach lookup $SERVER $(agent-id identity show 2>/dev/null | grep DID | awk '{print $2}')
```
