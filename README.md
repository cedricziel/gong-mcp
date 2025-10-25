# Gong MCP Server

A Model Context Protocol (MCP) server that provides access to Gong calls and data via **tools** (for search) and **resources** (for direct access).

## Features

- **Flexible call search** via `search_calls` tool with comprehensive filters
- **Direct data access** via resources (transcripts, users, status)
- **Pagination support** with cursor-based navigation
- Built with the official Rust MCP SDK (rmcp v0.8)
- Docker container support for easy deployment

## Prerequisites

- Gong API credentials (Access Key and Access Key Secret)
- Gong Base URL for your organization

## Configuration

The server requires the following environment variables:

- `GONG_BASE_URL`: Your Gong API base URL (e.g., `https://api.gong.io`)
- `GONG_ACCESS_KEY`: Your Gong API access key
- `GONG_ACCESS_KEY_SECRET`: Your Gong API access key secret

## Installation

### Using Docker (Recommended)

Pull the pre-built image from GitHub Container Registry:

```bash
docker pull ghcr.io/cedricziel/gong-mcp:latest
```

Run the container:

```bash
docker run -it \
  -e GONG_BASE_URL="https://api.gong.io" \
  -e GONG_ACCESS_KEY="your-access-key" \
  -e GONG_ACCESS_KEY_SECRET="your-secret" \
  ghcr.io/cedricziel/gong-mcp:latest
```

### From Source

```bash
cargo build --release
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-access-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
./target/release/gong-mcp
```

## Using with Claude Desktop

Add the following to your Claude Desktop configuration file:

### macOS

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gong": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "GONG_BASE_URL=https://api.gong.io",
        "-e",
        "GONG_ACCESS_KEY=your-access-key",
        "-e",
        "GONG_ACCESS_KEY_SECRET=your-secret",
        "ghcr.io/cedricziel/gong-mcp:latest"
      ]
    }
  }
}
```

### Windows

Edit `%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gong": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "GONG_BASE_URL=https://api.gong.io",
        "-e",
        "GONG_ACCESS_KEY=your-access-key",
        "-e",
        "GONG_ACCESS_KEY_SECRET=your-secret",
        "ghcr.io/cedricziel/gong-mcp:latest"
      ]
    }
  }
}
```

### Linux

Edit `~/.config/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gong": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "GONG_BASE_URL=https://api.gong.io",
        "-e",
        "GONG_ACCESS_KEY=your-access-key",
        "-e",
        "GONG_ACCESS_KEY_SECRET=your-secret",
        "ghcr.io/cedricziel/gong-mcp:latest"
      ]
    }
  }
}
```

## Using with Claude Code (Cursor IDE)

Add to your Cursor settings:

1. Open Cursor Settings
2. Navigate to MCP Servers
3. Add a new server configuration:

```json
{
  "gong": {
    "command": "docker",
    "args": [
      "run",
      "-i",
      "--rm",
      "-e",
      "GONG_BASE_URL=https://api.gong.io",
      "-e",
      "GONG_ACCESS_KEY=your-access-key",
      "-e",
      "GONG_ACCESS_KEY_SECRET=your-secret",
      "ghcr.io/cedricziel/gong-mcp:latest"
    ]
  }
}
```

## Available Capabilities

Once configured, the server exposes:

### Tools

**`search_calls`** - Flexible call search with optional filters:

- `from_date_time` (string): ISO 8601 start date
- `to_date_time` (string): ISO 8601 end date
- `workspace_id` (string): Filter by workspace
- `call_ids` (array): Specific call IDs
- `primary_user_ids` (array): Filter by user/host
- `cursor` (string): Pagination cursor

All parameters are optional. Returns calls with pagination support.

### Resources

**Static:**

- `gong://status` - Configuration status and health check
- `gong://users` - List of users in your Gong workspace

**Dynamic (templates):**

- `gong://calls/{callId}/transcript` - Get transcript for a specific call

## Usage Examples

### Searching for Calls

Ask Claude to search for calls with natural language:

- "Show me calls from last week"
- "Find calls where user ID 12345 participated in January"
- "Get calls from workspace W789"

Claude will use the `search_calls` tool with appropriate parameters:

```json
{
  "name": "search_calls",
  "arguments": {
    "from_date_time": "2024-01-01T00:00:00Z",
    "to_date_time": "2024-01-31T23:59:59Z",
    "primary_user_ids": ["12345"]
  }
}
```

### Accessing Transcripts

Once you have a call ID from search results, ask for the transcript:

- "Show me the transcript for call ABC123"

Claude will access: `gong://calls/ABC123/transcript`

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running locally

```bash
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-access-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
cargo run
```

## Dependencies

- [gong-rs](https://github.com/cedricziel/gong-rs) - Rust client library for the Gong API
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Official Rust SDK for Model Context Protocol

## License

Apache-2.0
