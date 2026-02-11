# Handshake: Secure Connection Establishment

## The Problem

After discovering an agent via agent-reach, you need to verify their identity before communicating. But the handshake phase is dangerous:

- **Prompt injection**: Malicious agent sends text that manipulates the LLM
- **Impersonation**: Agent claims to be someone they're not
- **Man-in-the-middle**: Attacker intercepts and relays

The handshake must be **cryptographically secure** and **LLM-free**.

## Principle: Runtime Handles Handshake

```
┌─────────────────────────────────────────────────┐
│                  RUNTIME LAYER                  │
│              (code, not LLM)                    │
│                                                 │
│   1. Connect to endpoint                        │
│   2. Exchange handshake (pure crypto data)      │
│   3. Verify signatures                          │
│   4. Establish verified channel                 │
│                                                 │
│   ─────────────── GATE ─────────────────────    │
│   Only passes if cryptographic verification OK │
│                                                 │
└─────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────┐
│                   LLM LAYER                     │
│                                                 │
│   Agent receives: "Verified connection with     │
│                    did:key:z6MkB..."            │
│                                                 │
│   Now safe to exchange messages                 │
│                                                 │
└─────────────────────────────────────────────────┘
```

The LLM **never sees** raw handshake data. It only receives a verified identity after the runtime completes authentication.

## Handshake Protocol

### Message Types

Strictly typed. No free-form text fields.

#### 1. Init (A → B)

```json
{
  "type": "handshake_init",
  "version": 1,
  "did": "did:key:z6MkA...",
  "challenge": "base64_32_random_bytes",
  "timestamp": 1707636600
}
```

#### 2. Response (B → A)

```json
{
  "type": "handshake_response",
  "version": 1,
  "did": "did:key:z6MkB...",
  "challenge": "base64_32_random_bytes",
  "challenge_response": "base64_signature_of_A_challenge",
  "timestamp": 1707636601
}
```

#### 3. Complete (A → B)

```json
{
  "type": "handshake_complete",
  "challenge_response": "base64_signature_of_B_challenge"
}
```

#### 4. Verified

Both sides now have:
- Verified the other's DID (they control the private key)
- Mutual authentication complete
- Safe to proceed to application protocol

### Field Constraints

| Field | Type | Constraints |
|-------|------|-------------|
| type | string | Enum: `handshake_init`, `handshake_response`, `handshake_complete` |
| version | integer | Must be supported version |
| did | string | Valid DID format, must match signer |
| challenge | string | Base64, exactly 32 bytes decoded |
| challenge_response | string | Base64, valid Ed25519 signature |
| timestamp | integer | Unix seconds, within 5 minute window |

**No other fields allowed.** Messages with extra fields are rejected.

### Validation Rules

1. **Schema validation**: Message must match exact schema
2. **DID format**: Must be valid `did:key` with Ed25519 key
3. **Timestamp window**: Must be within ±5 minutes of local time
4. **Signature verification**: Challenge response must be valid signature by claimed DID
5. **Challenge entropy**: Must be cryptographically random

### Failure Modes

Any validation failure = connection terminated. No error details sent (prevents oracle attacks).

```json
{
  "type": "handshake_error",
  "code": "verification_failed"
}
```

Then close connection.

## Security Properties

### What This Achieves

- **Mutual authentication**: Both agents prove they control their DIDs
- **Replay protection**: Timestamps + random challenges
- **No prompt injection surface**: No text fields for LLM to interpret
- **Impersonation prevention**: Can't fake signatures without private key

### What This Doesn't Cover

- **Authorization**: Knowing WHO they are, not WHAT they can do
- **Message encryption**: Relies on transport (TLS/WSS)
- **Ongoing verification**: Per-message signatures are separate concern

## Flow Example

```
Agent A                                          Agent B
   │                                                │
   │  { type: handshake_init,                       │
   │    did: "did:key:z6MkA...",                    │
   │    challenge: "abc123...",                     │
   │    timestamp: 1707636600 }                     │
   │───────────────────────────────────────────────▶│
   │                                                │ Verify A's DID format
   │                                                │ Store challenge
   │                                                │
   │  { type: handshake_response,                   │
   │    did: "did:key:z6MkB...",                    │
   │    challenge: "xyz789...",                     │
   │    challenge_response: sign(A_challenge, B_key)│
   │    timestamp: 1707636601 }                     │
   │◀───────────────────────────────────────────────│
   │                                                │
   │ Verify B's signature on A's challenge          │
   │ B is authentic ✓                               │
   │                                                │
   │  { type: handshake_complete,                   │
   │    challenge_response: sign(B_challenge, A_key)│
   │  }                                             │
   │───────────────────────────────────────────────▶│
   │                                                │ Verify A's signature
   │                                                │ A is authentic ✓
   │                                                │
   │           ═══ VERIFIED CHANNEL ═══             │
   │                                                │
   │  Now safe to exchange A2A messages             │
   │◀══════════════════════════════════════════════▶│
```

## Integration with agent-id

The handshake uses agent-id primitives:

```rust
// Verify challenge response
let valid = agent_id::verify(
    &their_did,
    &my_challenge,
    &their_signature
)?;
```

agent-id provides:
- DID parsing and validation
- Ed25519 signature creation/verification
- Key derivation from DIDs

## After Handshake

Once verified, the runtime notifies the LLM layer:

```
System: Verified connection established.
  Peer: did:key:z6MkB...
  Endpoint: wss://192.168.1.50:8080
  Protocol: A2A v1

You may now communicate with this agent.
```

The LLM knows WHO it's talking to (verified DID) but never saw the handshake itself.

## Future Considerations

- **Session tokens**: Avoid re-handshaking on reconnect
- **Key rotation**: Handle DID key updates gracefully
- **Delegation verification**: Check if agent is acting on behalf of another
- **Capability negotiation**: Exchange supported protocols post-handshake
