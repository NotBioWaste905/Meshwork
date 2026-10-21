pub mod auth;
pub mod backend;
pub mod graph;
pub mod model;
pub mod tools;

use std::sync::Arc;

use axum::{Json, Router, middleware, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

/// Router with `/mcp` behind bearer auth and an open `/health`.
pub fn app(backend: backend::Backend, tokens: Vec<String>, extra_hosts: Vec<String>) -> Router {
    let mut config = StreamableHttpServerConfig::default();
    config.allowed_hosts.extend(extra_hosts);
    let service: StreamableHttpService<tools::Meshwork, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(tools::Meshwork::new(backend.clone())),
            Default::default(),
            config,
        );
    let protected =
        Router::new()
            .nest_service("/mcp", service)
            .layer(middleware::from_fn_with_state(
                Arc::new(tokens),
                auth::require_token,
            ));
    Router::new()
        .route(
            "/health",
            get(|| async {
                Json(serde_json::json!({ "status": "healthy", "service": "meshwork-mcp" }))
            }),
        )
        .merge(protected)
}
