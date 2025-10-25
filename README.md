# Gong MCP Server

A Model Context Protocol (MCP) server that provides access to Gong calls and data via resources.

## Features

- **Resource-based access** to Gong API data
- Exposes Gong calls, users, and other resources
- Built with the official Rust MCP SDK
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

## Available Resources

Once configured, the server exposes the following resources:

- `gong://status` - Configuration status and health check
- `gong://calls` - List of recent calls from Gong
- `gong://users` - List of users in your Gong workspace

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
