# Architecture

## Overview

```
┌──────────────┐         ┌──────────────┐
│   Agent A    │         │   Agent B    │
│  (OpenClaw)  │         │  (OpenClaw)  │
└──────┬───────┘         └───────┬──────┘
       │                         │
       │ WebSocket               │ WebSocket
       │                         │
       ▼                         ▼
┌─────────────────────────────────────────┐
│             agent-relay                  │
│                                          │
│  ┌─────────────┐    ┌─────────────────┐ │
│  │  Registry   │    │  Message Queue  │ │
│  │ DID→status  │    │   per agent     │ │
│  └─────────────┘    └─────────────────┘ │
└─────────────────────────────────────────┘
```

## Components

### 1. Registry

Maps DIDs to connection state:

```
did:key:z6MkA... → { 
  status: "online",
  connected_at: "2026-02-11T05:00:00Z",
  endpoint: null  // using relay
}

did:key:z6MkB... → {
  status: "offline",
  last_seen: "2026-02-11T04:00:00Z",
  endpoint: "https://agent-b.example.com"  // direct A2A
}
```

### 2. Message Queue

Per-agent queue for offline delivery:

- Messages held until agent connects
- TTL on messages (e.g., 7 days)
- Delivered in order when agent reconnects

### 3. WebSocket Connections

Agents maintain persistent connection to relay:

- Real-time message delivery
- Presence updates
- Heartbeat to detect disconnection

## Flows

### Registration

```
Agent                           Relay
  │                               │
  │  POST /register               │
  │  { did, signature }           │
  │──────────────────────────────▶│
  │                               │ Verify signature
  │                               │ Store in registry
  │        { ok: true }           │
  │◀──────────────────────────────│
  │                               │
  │  WS /connect                  │
  │  { did, signature }           │
  │══════════════════════════════▶│
  │                               │ Mark online
  │     connection established    │ Deliver queued messages
  │◀══════════════════════════════│
```

### Lookup

```
Agent A                         Relay
  │                               │
  │  GET /lookup/{did}            │
  │──────────────────────────────▶│
  │                               │
  │  { status, endpoint? }        │
  │◀──────────────────────────────│
```

### Send Message

```
Agent A                         Relay                          Agent B
  │                               │                               │
  │  POST /send                   │                               │
  │  { to: did, message, sig }    │                               │
  │──────────────────────────────▶│                               │
  │                               │  If B online:                 │
  │                               │  ═══════════════════════════▶ │
  │                               │  WS push                      │
  │                               │                               │
  │                               │  If B offline:                │
  │                               │  Queue message                │
  │        { queued: true }       │                               │
  │◀──────────────────────────────│                               │
```

## Future: Federation

Relays can peer with each other:

```
Agent A ──▶ Relay 1 ──▶ Relay 2 ──▶ Agent B
            (A's home)   (B's home)
```

DID could include relay hint:
```
did:key:z6MkA...?relay=relay1.example.com
```

Or relays maintain a DHT of DID→home-relay mappings.

## Tech Stack (Proposed)

- **Runtime**: Rust (tokio) or Node.js
- **Database**: Redis (fast, built-in pub/sub) or Postgres
- **WebSocket**: tokio-tungstenite or ws
- **Deployment**: Single VPS to start, horizontal scale later
