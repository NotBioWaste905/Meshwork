pub mod auth;
pub mod backend;
pub mod graph;
pub mod model;
pub mod tools;
pub mod view;

use axum::{Json, Router, middleware, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

/// Router with `/mcp` behind actor bearer tokens (checked by the backend) and an open `/health`.
pub fn app(backend: backend::Backend, extra_hosts: Vec<String>) -> Router {
    let mut config = StreamableHttpServerConfig::default();
    config.allowed_hosts.extend(extra_hosts);
    let service: StreamableHttpService<tools::Meshwork, LocalSessionManager> =
        StreamableHttpService::new(|| Ok(tools::Meshwork::new()), Default::default(), config);
    let protected = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(backend, auth::require_actor));
    Router::new()
        .route(
            "/health",
            get(|| async {
                Json(serde_json::json!({ "status": "healthy", "service": "meshwork-mcp" }))
            }),
        )
        .merge(protected)
}
