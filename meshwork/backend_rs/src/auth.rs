//! Actor bearer tokens: who is making a request.
//!
//! A token is `mw_` + 64 hex characters (two v4 UUIDs, 244 random bits). Only its SHA-256 is
//! stored, as `(:Token {hash})-[:OF]->(:Actor)`, so a database dump does not leak usable
//! tokens; with that much entropy a plain hash is enough, no salt or slow KDF. Tokens are issued
//! by the `meshwork-token` CLI, not over HTTP: there is no login, and while the API itself is
//! open, an endpoint minting tokens would hand anyone any identity.
//!
//! Requests without a token are still served, as nobody (`Caller(None)`); requiring one is a
//! separate step. A token that is presented but unknown or malformed is a 401.

use axum::{
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use falkordb::AsyncGraph;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, Result},
    schemas::{Actor, TokenInfo},
    AppState,
};

const TOKEN_PREFIX: &str = "mw_";

/// 401 text for a bearer value that is not a token at all. The usual mistake is passing the
/// token's id (a UUID, as `meshwork-token list` prints it) instead of the token.
pub const NOT_A_TOKEN: &str = "Not a Meshwork token: tokens start with 'mw_'. \
     A token id (a UUID, as `meshwork-token list` shows) is not the token; \
     issue a new one with `meshwork-token create <actor>`";

/// The actor a request comes from, put into the request extensions by [`identify`].
#[derive(Clone, Debug)]
pub struct Caller(pub Option<Actor>);

/// A new random token. Never stored as is.
fn generate_token() -> String {
    format!(
        "{}{}{}",
        TOKEN_PREFIX,
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    )
}

pub fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// Issues a token for the actor given by ID or name. Returns the token, to be shown once, and
/// its ID for revoking.
pub async fn issue_token(
    graph: &mut AsyncGraph,
    actor: &str,
    label: &str,
) -> Result<(String, String, Actor)> {
    let actor_id = db::resolve_actors(graph, &[actor.to_string()])
        .await?
        .remove(0);
    let token = generate_token();
    let token_id = Uuid::new_v4().to_string();
    db::create_token(graph, &token_id, &hash_token(&token), &actor_id, label).await?;
    let actor = db::get_actor(graph, &actor_id).await?;
    Ok((token, token_id, actor))
}

/// Tokens of one actor (by ID or name), or of everyone.
pub async fn list_tokens(graph: &mut AsyncGraph, actor: Option<&str>) -> Result<Vec<TokenInfo>> {
    let actor_id = match actor {
        Some(actor) => Some(
            db::resolve_actors(graph, &[actor.to_string()])
                .await?
                .remove(0),
        ),
        None => None,
    };
    db::list_tokens(graph, actor_id.as_deref()).await
}

pub async fn revoke_token(graph: &mut AsyncGraph, token_id: &str) -> Result<()> {
    if db::revoke_token(graph, token_id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound(format!("Token {} not found", token_id)))
    }
}

/// The actor behind an `Authorization` header value: `None` without a header, 401 for anything
/// but a known `Bearer` token.
pub async fn authenticate(graph: &mut AsyncGraph, header: Option<&str>) -> Result<Option<Actor>> {
    let Some(value) = header else {
        return Ok(None);
    };
    let token = value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| t.starts_with(TOKEN_PREFIX))
        .ok_or_else(|| AppError::Unauthorized(NOT_A_TOKEN.to_string()))?;
    match db::actor_by_token_hash(graph, &hash_token(token)).await? {
        Some(actor) => Ok(Some(actor)),
        None => Err(AppError::Unauthorized(
            "Unknown or revoked token; issue a new one with `meshwork-token create <actor>`"
                .to_string(),
        )),
    }
}

/// Middleware: resolves the bearer token of every `/v0/` request into a [`Caller`]. Runs after
/// the connection check, so the lookup has a live socket.
pub async fn identify<B>(
    State(state): State<AppState>,
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    if request.uri().path().starts_with("/v0/") {
        let header = request
            .headers()
            .get(header::AUTHORIZATION)
            .map(|v| v.to_str().unwrap_or("?"));
        let mut graph = state.graph.clone();
        match authenticate(&mut graph, header).await {
            Ok(actor) => {
                request.extensions_mut().insert(Caller(actor));
            }
            Err(e) => return e.into_response(),
        }
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_hashed() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        assert!(a.starts_with(TOKEN_PREFIX));
        assert_eq!(a.len(), TOKEN_PREFIX.len() + 64);
        assert_eq!(hash_token(&a), hash_token(&a));
        assert_eq!(hash_token(&a).len(), 64);
        assert_ne!(hash_token(&a), hash_token(&b));
    }
}
