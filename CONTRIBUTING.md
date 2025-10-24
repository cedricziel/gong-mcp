# Contributing to Gong MCP Server

Thank you for your interest in contributing to the Gong MCP Server!

## Development Setup

1. **Install Rust**: Make sure you have Rust 1.90 or later installed.
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/cedricziel/gong-mcp.git
   cd gong-mcp
   ```

3. **Build the project**:
   ```bash
   cargo build
   ```

4. **Run tests**:
   ```bash
   cargo test
   ```

## Development Workflow

1. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** and ensure they follow the project's coding standards:
   ```bash
   cargo fmt
   cargo clippy
   ```

3. **Run tests** to ensure nothing is broken:
   ```bash
   cargo test
   ```

4. **Commit your changes** with a descriptive message:
   ```bash
   git commit -m "feat: add new feature"
   ```

5. **Push to your fork** and create a pull request.

## Code Standards

- Follow Rust's standard formatting (enforced by `cargo fmt`)
- Address all `cargo clippy` warnings
- Add tests for new functionality
- Update documentation as needed

## Testing

Run the full test suite:
```bash
cargo test
```

Test the binary manually:
```bash
GONG_BASE_URL="https://api.gong.io" \
GONG_ACCESS_KEY="your-key" \
GONG_ACCESS_KEY_SECRET="your-secret" \
cargo run
```

## Docker Testing

Build and test the Docker image:
```bash
docker build -t gong-mcp:dev .
docker run -it --rm \
  -e GONG_BASE_URL="https://api.gong.io" \
  -e GONG_ACCESS_KEY="your-key" \
  -e GONG_ACCESS_KEY_SECRET="your-secret" \
  gong-mcp:dev
```

## Pull Request Process

1. Update the README.md with details of changes if applicable
2. Ensure all tests pass
3. Update the version number following [Semantic Versioning](https://semver.org/)
4. The PR will be merged once you have the sign-off of a maintainer

## Questions?

Feel free to open an issue for any questions or concerns.
