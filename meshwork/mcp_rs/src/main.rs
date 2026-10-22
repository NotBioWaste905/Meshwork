use std::env;

use mcp_rs::{app, backend::Backend};

fn list_env(name: &str) -> Vec<String> {
    env::var(name)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    if env::var_os("MCP_TOKENS").is_some() {
        tracing::warn!(
            "MCP_TOKENS is ignored: clients use actor tokens from `meshwork-token create <actor>`"
        );
    }
    let backend_url = env::var("BACKEND_URL").unwrap_or_else(|_| "http://localhost:8000".into());
    let bind = env::var("MCP_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let router = app(Backend::new(&backend_url), list_env("MCP_ALLOWED_HOSTS"));
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind");
    tracing::info!("meshwork-mcp on {bind}, backend {backend_url}");
    axum::serve(listener, router).await.expect("serve");
}
