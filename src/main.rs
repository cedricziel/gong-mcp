use anyhow::Result;
use clap::Parser;
use gong_mcp::GongServer;
use rmcp::{ServiceExt, transport::stdio};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService,
    session::local::LocalSessionManager,
};
use tracing_subscriber::EnvFilter;

// Axum is brought in by rmcp's transport-streamable-http-server feature
use axum;

/// Gong MCP Server - Access Gong calls and data via Model Context Protocol
#[derive(Parser, Debug)]
#[command(name = "gong-mcp")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Transport mode: stdio or http
    #[arg(long, default_value = "stdio", value_parser = ["stdio", "http"])]
    mode: String,

    /// Host address to bind to (HTTP mode only)
    #[arg(long, default_value_t = default_host())]
    host: String,

    /// Port to bind to (HTTP mode only)
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

/// Determines default host based on environment
/// Returns 0.0.0.0 in Docker, 127.0.0.1 otherwise
fn default_host() -> String {
    // Check for common Docker environment indicators
    let in_docker = std::env::var("DOCKER_ENV").is_ok()
        || std::path::Path::new("/.dockerenv").exists()
        || std::fs::read_to_string("/proc/1/cgroup")
            .map(|s| s.contains("docker"))
            .unwrap_or(false);

    if in_docker {
        "0.0.0.0".to_string()
    } else {
        "127.0.0.1".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    tracing::info!("Starting Gong MCP server in {} mode", cli.mode);

    // Create the Gong server
    let server = GongServer::new();

    match cli.mode.as_str() {
        "stdio" => {
            tracing::info!("Using stdio transport");
            // Serve using stdio transport
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!("Server error: {:?}", e);
            })?;

            // Wait for the service to complete
            service.waiting().await?;
        }
        "http" => {
            let addr: std::net::SocketAddr = format!("{}:{}", cli.host, cli.port)
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid host:port combination: {}", e))?;

            tracing::info!("Using Streamable HTTP transport on http://{}", addr);
            tracing::info!("HTTP endpoint: http://{}/mcp", addr);

            // Create the streamable HTTP service
            let service = StreamableHttpService::new(
                move || Ok(server.clone()),
                LocalSessionManager::default().into(),
                Default::default(),
            );

            // Create router and nest service under /mcp
            let router = axum::Router::new().nest_service("/mcp", service);

            // Bind to address
            let listener = tokio::net::TcpListener::bind(addr).await?;
            tracing::info!("HTTP server listening on {}", addr);

            // Serve with graceful shutdown
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::signal::ctrl_c().await.ok();
                })
                .await?;
        }
        _ => {
            anyhow::bail!("Invalid mode: {}. Must be 'stdio' or 'http'", cli.mode);
        }
    }

    Ok(())
}
