//! FalkorDB connection setup and recovery.
//!
//! The client keeps a single multiplexed socket wrapped in redis'
//! `ConnectionManager`. When FalkorDB restarts, the manager reconnects on its
//! own, but only after a command has failed on the dead socket. To keep that
//! failure away from the real queries, every API request first sends a cheap
//! read-only ping: if it hits a dead connection it is retried once (that second
//! attempt waits for the reconnect), so the handler's queries — writes included —
//! run on a live socket and are never retried themselves.

use axum::{
    extract::State,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use falkordb::{
    AsyncGraph, FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo, FalkorDBError,
};
use std::num::NonZeroU8;
use tracing::{info, warn};

use crate::{
    error::{db_err, AppError, Result},
    AppState,
};

/// Builds the async client. One socket: it is multiplexed, so concurrent
/// requests share it, and the per-request ping always checks the socket the
/// queries will use.
pub async fn connect(uri: &str) -> Result<FalkorAsyncClient> {
    let info: FalkorConnectionInfo = uri.try_into().map_err(db_err)?;
    FalkorClientBuilder::new_async()
        .with_connection_info(info)
        .with_num_connections(NonZeroU8::MIN)
        .build()
        .await
        .map_err(db_err)
}

fn is_connection_error(e: &FalkorDBError) -> bool {
    matches!(
        e,
        FalkorDBError::ConnectionDown
            | FalkorDBError::NoConnection
            | FalkorDBError::EmptyConnection
    )
}

/// Makes sure the connection is alive, reconnecting if FalkorDB was restarted.
pub async fn ensure_connection(graph: &mut AsyncGraph) -> Result<()> {
    for attempt in 1..=2 {
        match graph.ro_query("RETURN 1").execute().await {
            // Any reply from the server — even an error such as "empty key" for a
            // graph that does not exist yet — means the socket works.
            Ok(_) => return Ok(()),
            Err(e) if !is_connection_error(&e) => return Ok(()),
            Err(e) if attempt == 1 => warn!("FalkorDB connection lost ({}), reconnecting", e),
            Err(e) => return Err(db_err(e)),
        }
    }
    unreachable!("the second attempt always returns")
}

/// Middleware: runs [`ensure_connection`] before every `/v0/` request.
pub async fn ensure_connection_layer<B>(
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    if request.uri().path().starts_with("/v0/") {
        let mut graph = state.graph.clone();
        if let Err(e) = ensure_connection(&mut graph).await {
            return e.into_response();
        }
    }
    next.run(request).await
}

/// Creates the lookup indexes every query relies on. Idempotent: an index
/// that already exists is not an error.
pub async fn ensure_indexes(graph: &mut AsyncGraph) -> Result<()> {
    for (label, q) in [
        ("Task", "CREATE INDEX FOR (t:Task) ON (t.id)"),
        ("Graph", "CREATE INDEX FOR (g:Graph) ON (g.id)"),
    ] {
        match graph.query(q).execute().await {
            Ok(_) => info!("Created index on {}.id", label),
            Err(e) if e.to_string().contains("already indexed") => {}
            Err(e) => return Err(AppError::Database(format!("index on {label}.id: {e}"))),
        }
    }
    Ok(())
}
