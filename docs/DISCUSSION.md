# Design Discussion

*Captured from initial brainstorm, 2026-02-11*

## The Problem We're Solving

A2A (Google's agent-to-agent protocol) handles communication *once connected*, but doesn't solve:

1. **Discovery**: How do I find an agent by their identity?
2. **Reachability**: Many agents are behind NAT, no public IP, ephemeral
3. **Offline delivery**: What if the agent isn't running right now?

agent-id gives agents DIDs. But DID alone doesn't tell you how to reach them.

## Key Insight

A2A's discovery assumes agents have:
- A domain
- A web server
- Public reachability at `/.well-known/agent-card.json`

Most agents don't have that. They're running on laptops, in containers, behind firewalls.

## The Solution: Relay Service

A service that bridges DID → reachability:

- **Registry**: Maps DIDs to status/endpoints
- **Relay**: Holds messages for unreachable agents
- **Real-time**: WebSocket for instant delivery

Think: **email for agents** — you don't need to be online when I send.

## Integration Points

### With agent-id
- All registrations signed with agent's DID key
- All messages signed for authenticity
- DID is the only identifier needed

### With A2A
- Once agents are connected via relay, they speak A2A
- Relay can return A2A-compatible Agent Card
- Relay is the *transport*, A2A is the *protocol*

### With OpenClaw
- MCP tools for relay operations
- Agent registers on startup
- Messages received and processed in conversation

## Centralized vs Decentralized

**Start centralized:**
- Ship fast
- Prove the concept
- One relay at agent-id.ai

**Design for federation:**
- Open protocol
- Anyone can run a relay
- Agents can specify their home relay

**Future decentralized (maybe):**
- DHT-based discovery
- No central service
- Complex, solve later

## Open Questions

1. **Message format**: Use A2A message format? Or simpler envelope?
2. **Authentication**: Just signatures, or full handshake per connection?
3. **Rate limiting**: How to prevent spam/abuse?
4. **Encryption**: E2E encryption, or rely on TLS + signatures?
5. **Persistence**: How long to queue offline messages?
6. **Multi-relay**: How do relays discover each other?

## Next Steps

1. Define wire protocol
2. Build minimal relay server
3. Add MCP tools for OpenClaw integration
4. Deploy at relay.agent-id.ai
5. Test with two OpenClaw agents
