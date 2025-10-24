# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial MCP server implementation using Rust MCP SDK
- Resource-based access to Gong API
- Docker container support with multi-stage build
- GitHub Actions workflow for building and publishing Docker images
- Comprehensive README with setup instructions for Claude Desktop and Claude Code
- Example configuration files
- Environment variable-based configuration (GONG_BASE_URL, GONG_ACCESS_KEY, GONG_ACCESS_KEY_SECRET)
- Resources:
  - `gong://status` - Configuration and health status
  - `gong://calls` - Gong calls (placeholder)
  - `gong://users` - Gong users (placeholder)

### Dependencies
- rmcp 0.8.3 - Official Rust MCP SDK
- gong-rs 0.0.1 - Gong API client
- tokio 1.48.0 - Async runtime
- anyhow 1.0.100 - Error handling
- tracing 0.1.41 - Logging

## [0.1.0] - 2025-10-24

### Added
- Initial release
- Basic MCP server structure
- Resource exposure framework
- Docker containerization
- CI/CD pipeline
