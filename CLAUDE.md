# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Model Context Protocol (MCP) server that provides access to Gong call data via **tools** (for search) and **resources** (for direct data access). Built with the Rust MCP SDK (rmcp v0.8), it acts as a bridge between the Gong API and LLM applications.

**Key Dependencies:**

- `rmcp` v0.8 - Official Rust MCP SDK
- `gong-rs` v0.0.1 - Rust client library for Gong API (early stage)
- `tokio` - Async runtime

**Architecture Pattern:**

- **Tools** = Dynamic operations with parameters (search, filters)
- **Resources** = Direct data access with stable URIs (transcripts, users, status)

## Common Commands

**Build:**

```bash
cargo build
cargo build --release
```

**Test:**

```bash
cargo test
cargo test -- --nocapture  # Show println! output
```

**Run locally (stdio mode - default):**

```bash
# Requires environment variables
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-access-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
cargo run
```

**Run locally (HTTP mode):**

```bash
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-access-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
cargo run -- --mode http --host 127.0.0.1 --port 8080
```

**Get CLI help:**

```bash
cargo run -- --help
```

**Linting:**

```bash
cargo clippy
cargo fmt --check
```

**Docker:**

```bash
# Build
docker build -t gong-mcp .

# Run (stdio mode)
docker run -i --rm \
  -e GONG_BASE_URL="https://api.gong.io" \
  -e GONG_ACCESS_KEY="your-key" \
  -e GONG_ACCESS_KEY_SECRET="your-secret" \
  gong-mcp

# Run (HTTP mode)
docker run -d \
  -p 8080:8080 \
  -e GONG_BASE_URL="https://api.gong.io" \
  -e GONG_ACCESS_KEY="your-key" \
  -e GONG_ACCESS_KEY_SECRET="your-secret" \
  gong-mcp \
  --mode http --host 0.0.0.0 --port 8080
```

## MCP Architecture

### Hybrid Tool + Resource Pattern

This server uses **tools** for dynamic search operations and **resources** for direct data access:

**In MCP:**

- **Tools** = Operations that require dynamic parameter construction (e.g., search with filters)
- **Resources** = Direct data access with stable URIs (e.g., transcripts, user lists)

**Why this design?**

- Search operations need flexible parameter passing → Tool (search_calls)
- Direct data access needs stable URIs → Resources (transcripts, users, status)

### Tools

**`search_calls` Tool:**
Flexible call search with optional filter parameters:

- `from_date_time` (string): ISO 8601 start date
- `to_date_time` (string): ISO 8601 end date
- `workspace_id` (string): Filter by workspace
- `call_ids` (array): Specific call IDs
- `primary_user_ids` (array): Filter by user/host
- `cursor` (string): Pagination cursor

**All parameters are optional.** If no filters provided, returns all available calls.

**Example LLM Usage:**

```json
{
  "name": "search_calls",
  "arguments": {
    "from_date_time": "2024-01-01T00:00:00Z",
    "to_date_time": "2024-01-31T23:59:59Z",
    "primary_user_ids": ["user123"]
  }
}
```

**Response Format:**

```json
{
  "calls": [...],
  "count": 25,
  "nextCursor": "eyJjdXJzb3IiOiJuZXh0In0=",
  "hasMore": true,
  "filters": {
    "from_date_time": "2024-01-01T00:00:00Z",
    "to_date_time": "2024-01-31T23:59:59Z",
    "primary_user_ids": ["user123"]
  }
}
```

### Resources

**Static Resources** (always listed):

- `gong://status` - Configuration status check
- `gong://users` - All users in workspace

**Dynamic Resources** (via templates):

- `gong://calls/{callId}/transcript` - Transcript for specific call

### URI Scheme Convention

All resources use the `gong://` URI scheme:

```
gong://status
gong://users
gong://calls/{callId}/transcript
```

URI parsing in `read_resource()` uses pattern matching with `strip_prefix()`/`strip_suffix()` for parameter extraction.

## Transport Configuration

The server supports two transport modes: **stdio** (default) and **Streamable HTTP**.

### stdio Transport (Default)

**Use Cases:**
- Claude Desktop integration
- Local development and testing
- Process-based MCP clients
- Docker containers with stdin/stdout communication

**Characteristics:**
- Uses standard input/output streams
- Simple, low-overhead communication
- Synchronous request/response pattern
- No network ports required

**Running:**
```bash
# Default mode
cargo run

# Explicit stdio mode
cargo run -- --mode stdio
```

### Streamable HTTP Transport

**Use Cases:**
- Web-based clients
- Remote access scenarios
- Cloud deployments
- HTTP-based integrations

**Characteristics:**
- MCP spec-compliant (2025-03-26)
- HTTP-based communication with bidirectional streaming
- Supports remote connections
- Single endpoint (simpler than deprecated SSE dual-endpoint pattern)
- Native HTTP/2 and HTTP/3 support
- Better connection resilience and recovery

**CLI Options:**
- `--mode http` - Enable Streamable HTTP transport
- `--host <address>` - Bind address (default: 127.0.0.1, or 0.0.0.0 in Docker)
- `--port <port>` - Port number (default: 8080)

**Endpoint:**
- `http://<host>:<port>/mcp` - Streamable HTTP endpoint

**Running:**
```bash
# Local development
cargo run -- --mode http --host 127.0.0.1 --port 8080

# Remote access
cargo run -- --mode http --host 0.0.0.0 --port 8080
```

**Docker Auto-Detection:**
The server automatically detects Docker environment and defaults to `0.0.0.0` binding when:
- `DOCKER_ENV` environment variable is set, OR
- `/.dockerenv` file exists, OR
- `/proc/1/cgroup` contains "docker"

**Implementation Details (src/main.rs):**
- CLI parsing via `clap` with derive macros
- Uses `transport-streamable-http-server` feature from rmcp 0.8
- `StreamableHttpService::new()` with service factory pattern
- `LocalSessionManager` for session handling
- Axum router nesting service under `/mcp` path
- Graceful shutdown via `tokio::signal::ctrl_c()`
- Single HTTP server (no dual endpoint complexity)

## Core Architecture

### Entry Point (src/main.rs)

Async main with CLI-based transport selection:

1. Initializes tracing to stderr
2. Parses CLI arguments (mode, host, port)
3. Creates `GongServer` instance
4. Conditionally serves using either:
   - **stdio transport**: Simple stdin/stdout communication
   - **HTTP transport**: Streamable HTTP with axum router nesting the MCP service, graceful shutdown support
5. Waits for service completion

### Server Implementation (src/lib.rs)

**`GongServer` struct:**

```rust
pub struct GongServer {
    config: Arc<Option<Configuration>>,
}
```

The `Arc<Option<Configuration>>` pattern provides:

- Thread-safe sharing across async operations
- Graceful handling of missing configuration
- Server remains functional even without full config (status resource still works)

### MCP Handler Implementation

**`ServerHandler` trait methods:**

- `get_info()` - Returns server metadata and capabilities (resources + tools)
- `list_resources()` - Lists available static resources
- `read_resource()` - Fetches resource contents
- `list_resource_templates()` - Lists dynamic resource templates
- `list_tools()` - Lists available tools (search_calls)
- `call_tool()` - Executes tool with parameters

### Configuration Management

**Environment Variables:**

- `GONG_BASE_URL` - Gong API base URL
- `GONG_ACCESS_KEY` - API access key
- `GONG_ACCESS_KEY_SECRET` - API secret

**Configuration State Handling:**
Always check `_is_configured()` before API calls. The status resource is always available regardless of configuration state to help with debugging.

## Data Transformation Strategy

The server acts as a **simplification layer** between complex Gong API responses and LLM-friendly JSON:

**Calls transformation (src/lib.rs:263-291):**

- Extracts essential fields from nested structures
- Flattens metadata for easier consumption
- Provides count and summary message

**Transcript transformation (src/lib.rs:401-473):**

- Extracts sentences with speaker information
- Computes metadata (speaker count, sentence count, monologue count)
- Flattens nested structures into simpler JSON

**Why this matters:** LLMs work better with flattened, simplified JSON rather than deeply nested API responses.

## Error Handling

Uses appropriate MCP error types with structured context:

```rust
// Not configured
McpError::invalid_request("not_configured", Some(json!({...})))

// Resource not found
McpError::resource_not_found("call_not_found", Some(json!({...})))

// Invalid parameters
McpError::invalid_params("invalid_uri", Some(json!({...})))

// API errors
McpError::internal_error("api_error", Some(json!({...})))
```

Always include structured JSON data with context in errors for debugging.

## Adding New Features

### Adding a Static Resource

1. Add to `list_resources()` return value
2. Add URI match arm in `read_resource()`
3. Check configuration with `_is_configured()`
4. Call Gong API via `gong-rs`
5. Transform response to simplified JSON
6. Return `ReadResourceResult` with formatted data

Example pattern:

```rust
"gong://new-resource" => {
    if !self._is_configured() {
        return Err(McpError::invalid_request("not_configured", None));
    }

    // Fetch from API
    let data = fetch_from_gong_api().await?;

    // Transform and return
    Ok(ReadResourceResult {
        contents: vec![ResourceContents::text(
            serde_json::to_string_pretty(&data).unwrap(),
            uri,
        )],
    })
}
```

### Adding a Resource Template

1. Add to `list_resource_templates()` return value
2. Add pattern matching in `read_resource()` catch-all section
3. Extract parameters from URI using `strip_prefix()`/`strip_suffix()`
4. Validate parameters (check for empty, invalid format)
5. Return appropriate error if invalid
6. Fetch data using parameters
7. Format and return response

See transcript implementation for complete example.

### Adding a Tool

1. Add tool definition to `list_tools()` return value
2. Add match arm in `call_tool()` for tool name
3. Extract parameters from `arguments` (Option<JsonObject>)
4. Validate parameters and return errors if invalid
5. Call appropriate API methods
6. Transform response to LLM-friendly JSON
7. Return `CallToolResult` with formatted content

Example pattern:

```rust
"my_tool" => {
    // Check configuration
    if !self._is_configured() {
        return Err(McpError::invalid_request("not_configured", None));
    }

    // Extract parameters
    let args = arguments.as_ref();
    let param1 = args
        .and_then(|a| a.get("param1"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Call API
    let data = fetch_data(param1).await?;

    // Return result
    Ok(CallToolResult {
        content: vec![Content::text(
            serde_json::to_string_pretty(&data).unwrap(),
        )],
        structured_content: None,
        is_error: None,
        meta: None,
    })
}
```

**When to use Tool vs Resource:**

- **Tool**: Dynamic operations with multiple parameters, search/filter operations
- **Resource**: Direct data access with stable URIs, browsable data

## API Integration Patterns

### Gong API Client Usage

The server uses `gong-rs` for all API calls:

```rust
// Calls API
use gong_rs::apis::calls_api;
let result = calls_api::list_calls_extensive(config, params).await?;

// Users API
use gong_rs::apis::users_api;
let result = users_api::list_users(config, params).await?;
```

**Current Implementation:**

- ✅ Flexible search with all Gong API filter parameters
- ✅ Cursor-based pagination support
- ✅ Tool-based architecture for dynamic queries
- ❌ No caching layer (fresh API calls every time)
- ⚠️ `gong-rs` is v0.0.1 (early stage, API may change)

### Async Patterns

Uses Tokio multi-threaded runtime:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ...
}
```

All MCP handler methods are async. Use `.await` for API calls.

## Docker Deployment

**Supported Transports:** Both stdio and Streamable HTTP modes are supported.

**Default behavior:**
- Defaults to stdio mode for backward compatibility
- Auto-detects Docker environment for HTTP host binding (uses 0.0.0.0)
- Port 8080 is exposed for HTTP mode

**Container behavior:**

- Expects environment variables at runtime (GONG_BASE_URL, GONG_ACCESS_KEY, GONG_ACCESS_KEY_SECRET)
- `DOCKER_ENV=1` is set to enable auto-detection
- Logs to stderr (captured by Docker)
- Runs as PID 1 in container
- Handles signals for graceful shutdown

**Running modes:**

```bash
# Stdio mode (default, for Claude Desktop)
docker run -i --rm \
  -e GONG_BASE_URL="..." \
  -e GONG_ACCESS_KEY="..." \
  -e GONG_ACCESS_KEY_SECRET="..." \
  gong-mcp

# HTTP mode (for web clients)
docker run -d \
  -p 8080:8080 \
  -e GONG_BASE_URL="..." \
  -e GONG_ACCESS_KEY="..." \
  -e GONG_ACCESS_KEY_SECRET="..." \
  gong-mcp \
  --mode http --host 0.0.0.0 --port 8080
```

## Logging and Debugging

Uses `tracing` crate with stderr output:

```rust
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_writer(std::io::stderr)
    .with_ansi(false)
    .init();
```

**Why stderr?** Stdio transport uses stdin/stdout for MCP protocol, so logs must go to stderr.

**Control log level:**

```bash
RUST_LOG=debug cargo run
RUST_LOG=gong_mcp=trace cargo run
```

## Security Model

**Authentication:**

- Credentials via environment variables (good practice)
- No additional authentication layer (trusts MCP host)
- Server assumes it's running in trusted environment

**Input validation:**

- URI parameters are validated (check for empty, format)
- No SQL injection risk (no database)
- API errors don't leak sensitive data

**Error messages:**

- Include context for debugging
- Don't expose credentials or secrets
- Structured JSON for programmatic handling

## Known Limitations

1. **No caching** - Every tool call makes fresh API call
2. **No rate limiting** - No protection against excessive API calls
3. **No subscription support** - Resources/tools don't notify on updates
4. **Early stage dependency** - `gong-rs` is v0.0.1, API may change

## MCP Specification Compliance

**Protocol Version:** Compliant with MCP spec 2025-03-26

**Capabilities Implemented:**

- ✓ Resources (static) - Status and users
- ✓ Resource templates (dynamic) - Call transcripts
- ✓ Tools - Flexible call search with parameters
- ✓ Stdio transport
- ✓ Streamable HTTP transport (spec-compliant, replaces deprecated SSE)
- ✓ Proper error handling
- ✓ Pagination support (cursor-based)

**Not Implemented (and why):**

- Prompts - Not needed for this use case
- Subscriptions - Future enhancement

## Troubleshooting

**Server starts but no resources listed:**

- Check environment variables are set correctly
- Call `gong://status` resource to check configuration

**API errors:**

- Verify Gong credentials are valid
- Check base URL is correct (<https://api.gong.io>)
- Review logs with `RUST_LOG=debug`

**Empty results:**

- Calls: Check if there are calls in last 7 days
- Transcripts: Verify call ID is correct
- Users: Check if workspace has users

**Docker issues:**

- Ensure environment variables passed with `-e` flag
- Check logs: `docker logs <container-id>`
- Verify image is up to date: `docker pull ghcr.io/cedricziel/gong-mcp:latest`

## Testing Strategy

**Unit tests** (src/lib.rs:526-620):

- Server creation and info
- URI parsing for transcripts
- Configuration detection
- Mock configuration testing

**Run specific test:**

```bash
cargo test test_transcript_uri_parsing -- --nocapture
```

**Test with real API:**
Set environment variables and use MCP inspector or Claude Desktop to test resources.

## Future Enhancement Ideas

1. **Pagination support** - Handle large result sets efficiently
2. **Configurable time ranges** - Allow dynamic date ranges for calls
3. **Caching layer** - Reduce API calls for frequently accessed data
4. **Rate limiting** - Protect against excessive API usage
5. **Subscription support** - Notify on new calls or updates
6. **More resources** - Meetings, recordings, analytics, etc.
7. **Resource templates for users** - `gong://users/{userId}` for individual user details
8. **Search functionality** - Search calls by keywords, participants, etc.

## Code Style

- Use semantic commits (as per project guidelines)
- Format with `cargo fmt` before committing
- Run `cargo clippy` to catch common issues
- Private helper methods prefixed with `_` (e.g., `_is_configured()`)
- Comprehensive error context in all error returns
