# agent-reach-mcp

MCP (Model Context Protocol) server for the agent-reach discovery registry.

Allows AI agents to register their endpoints, look up other agents, and manage their presence in the agent-reach registry - all through safe, high-level MCP tools.

## Prerequisites

You need an agent-id identity first. Generate one using [agent-id-mcp](https://github.com/AustinEral/agent-id-mcp):

```bash
# Identity is stored at ~/.config/agent-id/identity.json
```

## Installation

```bash
cargo install agent-reach-mcp
```

## Usage

Run as an MCP server (stdio transport):

```bash
agent-reach-mcp
```

### Environment Variables

- `REACH_REGISTRY_URL` - Override the default registry URL (default: `https://reach.agent-id.ai`)

## MCP Tools

### `reach_register`

Register your agent's endpoint in the discovery registry.

**Parameters:**
- `endpoint` (string): The endpoint URL where your agent can be reached

**Example:**
```json
{
  "name": "reach_register",
  "arguments": {
    "endpoint": "https://example.com/agent/inbox"
  }
}
```

### `reach_lookup`

Look up another agent's endpoint by their DID.

**Parameters:**
- `did` (string): The DID of the agent to look up

**Example:**
```json
{
  "name": "reach_lookup",
  "arguments": {
    "did": "did:key:z6MkkCZkbDtaJA44BnE36aczhKyrgTjixJu2uqHNPPLU5S6F"
  }
}
```

### `reach_deregister`

Remove your agent's registration from the registry.

**Parameters:** None

### `reach_status`

Check your current registration status.

**Parameters:** None

### `reach_whoami`

Show your agent's DID.

**Parameters:** None

## How It Works

The MCP server handles all authentication automatically:

1. **You call** `reach_register(endpoint)`
2. **Internally:**
   - Loads your identity from `~/.config/agent-id/identity.json`
   - Performs handshake authentication with the registry (hello → challenge → proof)
   - Registers your endpoint with the authenticated session
3. **You see:** "✓ Registered did:key:... at endpoint: ..."

The agent never needs to handle cryptographic operations, challenges, or session tokens directly.

## License

Apache-2.0
