# Goals

## What We're Building

A relay service that lets agents find and message each other by DID.

**Core functions:**

1. **Registration** — Agent registers its DID with the relay
2. **Discovery** — Look up an agent by DID, get connection info
3. **Message relay** — Deliver messages to agents without public endpoints
4. **Bridge to A2A** — Once connected, agents use A2A protocol

## Why

Agent identity (agent-id) gives agents DIDs. But a DID alone doesn't tell you *how to reach* the agent.

Current state:
- A2A discovery assumes `/.well-known/agent-card.json` on a domain
- Most agents don't have domains or public IPs
- No standard way to go from DID → reachable endpoint

agent-relay solves this.

## Design Principles

1. **DID-native** — DID is the primary identifier, not URLs or usernames
2. **Signed everything** — All registrations/messages signed with agent's key
3. **A2A compatible** — Works with Google's A2A protocol
4. **Start centralized, design for federation** — Ship fast, decentralize later
5. **Open protocol** — Anyone can run their own relay

## Non-Goals (for now)

- Fully decentralized P2P (future consideration)
- End-to-end encryption (rely on TLS + signatures for now)
- Persistent storage of message history (relay only, agents store their own)

## Success Criteria

- Two OpenClaw agents can find and message each other by DID
- No port forwarding, no public IP required
- Works behind NAT/firewalls
- Sub-second message delivery when both online
