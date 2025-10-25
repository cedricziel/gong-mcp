# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Model Context Protocol (MCP) server that provides access to Gong call data and transcripts via resources. Built with the Rust MCP SDK (rmcp v0.8), it acts as a bridge between the Gong API and LLM applications.

**Key Dependencies:**
- `rmcp` v0.8 - Official Rust MCP SDK
- `gong-rs` v0.0.1 - Rust client library for Gong API (early stage)
- `tokio` - Async runtime

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

**Run locally:**
```bash
# Requires environment variables
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-access-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
cargo run
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

# Run
docker run -it \
  -e GONG_BASE_URL="https://api.gong.io" \
  -e GONG_ACCESS_KEY="your-key" \
  -e GONG_ACCESS_KEY_SECRET="your-secret" \
  gong-mcp
```

## MCP Architecture

### Resource-Based Design Pattern

This server correctly uses **resources** (not tools) for all data access. In MCP:
- **Resources** = Read-only data that LLMs can reference (like APIs or databases)
- **Tools** = Actions that modify state or execute commands

We expose Gong data as resources since we're providing read-only access to call data.

### Resource Types

**Static Resources** (always listed):
- `gong://status` - Configuration status check
- `gong://calls` - Recent calls from last 7 days
- `gong://users` - All users in workspace

**Dynamic Resources** (via templates):
- `gong://calls/{callId}/transcript` - Transcript for specific call

### URI Scheme Convention

All resources use the `gong://` URI scheme:
```
gong://status
gong://calls
gong://users
gong://calls/{callId}/transcript
```

URI parsing in `read_resource()` uses pattern matching with `strip_prefix()`/`strip_suffix()` for parameter extraction.

## Core Architecture

### Entry Point (src/main.rs)

Simple async main that:
1. Initializes tracing to stderr (required for stdio transport)
2. Creates `GongServer` instance
3. Serves using stdio transport
4. Waits for service completion

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
- `get_info()` - Returns server metadata and capabilities
- `list_resources()` - Lists available static resources
- `read_resource()` - Fetches resource contents
- `list_resource_templates()` - Lists dynamic resource templates

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

See transcript implementation (src/lib.rs:362-490) for complete example.

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

**Current limitations:**
- 7-day hardcoded time range for calls (line 67)
- No pagination support
- No caching layer
- `gong-rs` is v0.0.1 (early stage, API may change)

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

**Transport:** Stdio only (not SSE) - appropriate for Docker deployment where server runs as a subprocess.

**Container behavior:**
- Expects environment variables at runtime
- Logs to stderr (captured by Docker)
- Runs as PID 1 in container
- Handles signals for graceful shutdown

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

1. **No pagination support** - Returns all results from API calls (may be slow for large datasets)
2. **Hardcoded 7-day time range** - Calls are fetched from last 7 days only
3. **No caching** - Every resource read makes fresh API call
4. **No rate limiting** - No protection against excessive API calls
5. **No subscription support** - Resources don't notify on updates
6. **Early stage dependency** - `gong-rs` is v0.0.1, API may change

## MCP Specification Compliance

**Protocol Version:** V_2024_11_05

**Capabilities Implemented:**
- ✓ Resources (static)
- ✓ Resource templates (dynamic)
- ✓ Stdio transport
- ✓ Proper error handling

**Not Implemented (and why):**
- Tools - Not needed (only read operations)
- Prompts - Not needed for this use case
- Subscriptions - Future enhancement
- SSE transport - Not needed for local/Docker deployment

## Troubleshooting

**Server starts but no resources listed:**
- Check environment variables are set correctly
- Call `gong://status` resource to check configuration

**API errors:**
- Verify Gong credentials are valid
- Check base URL is correct (https://api.gong.io)
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
