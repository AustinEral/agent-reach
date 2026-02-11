# agent-relay

DID-based discovery and message relay for AI agents.

## The Problem

Agents have identities (DIDs via [agent-id](https://github.com/AustinEral/agent-id)), but how do they find and reach each other?

- Most agents don't have public IPs or domains
- NAT, firewalls, ephemeral environments
- A2A assumes you already have an endpoint URL

## The Solution

A relay service that bridges DID → reachability:

1. **Register**: Agent signs "I'm `did:key:z6Mk...`, relay for me"
2. **Lookup**: Query "where is `did:key:z6Mk...`?"
3. **Relay**: Messages held and delivered when agent connects

Once connected, agents communicate via [A2A protocol](https://github.com/google/A2A).

## How It Fits

```
┌─────────────────────────────────┐
│            A2A                  │  ← task/message protocol
├─────────────────────────────────┤
│       agent-relay               │  ← discovery + routing
├─────────────────────────────────┤
│         agent-id                │  ← identity
└─────────────────────────────────┘
```

## Status

🚧 **Early development** — see [docs/](docs/) for design discussions.

## Documentation

- [GOALS.md](docs/GOALS.md) — What we're building
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) — How it works
- [PROTOCOL.md](docs/PROTOCOL.md) — Wire protocol (coming soon)

## Related Projects

- [agent-id](https://github.com/AustinEral/agent-id) — Cryptographic identity for agents
- [agent-id-mcp](https://github.com/AustinEral/agent-id-mcp) — MCP server for agent-id
- [Google A2A](https://github.com/google/A2A) — Agent-to-agent communication protocol

## License

Apache-2.0
