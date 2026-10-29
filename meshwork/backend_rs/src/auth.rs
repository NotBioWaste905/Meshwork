//! Actor bearer tokens: who is making a request.
//!
//! A token is `mw_` + 64 hex characters (two v4 UUIDs, 244 random bits). Only its SHA-256 is
//! stored, as `(:Token {hash})-[:OF]->(:Actor)`, so a database dump does not leak usable
//! tokens; with that much entropy a plain hash is enough, no salt or slow KDF. A first token is
//! issued only by the `meshwork-token` CLI, never over HTTP: there is no login, and while the API
//! itself is open, an endpoint minting tokens for anyone would hand out any identity. Over HTTP
//! an actor can only manage its own tokens (`/v0/me/tokens`), which needs one of them already.
//!
//! Every `/v0/` request needs a token (401 without one), unless the backend runs with
//! `MESHWORK_ALLOW_ANONYMOUS=true`: then requests without a token are served as nobody
//! (`Caller(None)`), the compatibility mode for local development and old clients. A token that
//! is presented but unknown, revoked, expired or malformed is a 401 either way. `/` and `/health`
//! stay open.
//!
//! Tokens are looked up by their hash, an indexed equality match in FalkorDB, never compared
//! with a secret in Rust: a timing side channel would at best reveal the prefix of a SHA-256,
//! which does not help to find a token, so constant-time comparison is moot here.
//!
//! A token may have an expiry (`expires_at`, ms since the epoch); without one it lives until it
//! is revoked. Failed attempts are not rate-limited here: 244 random bits cannot be guessed,
//! and limiting by client address belongs in front of the backend (reverse proxy), together
//! with TLS.

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
    schemas::{Actor, ActorKind, TokenInfo},
    AppState,
};

const TOKEN_PREFIX: &str = "mw_";

/// 401 text for a bearer value that is not a token at all. The usual mistake is passing the
/// token's id (a UUID, as `meshwork-token list` prints it) instead of the token.
pub const NOT_A_TOKEN: &str = "Not a Meshwork token: tokens start with 'mw_'. \
     A token id (a UUID, as `meshwork-token list` shows) is not the token; \
     issue a new one with `meshwork-token create <actor>`";

/// 401 text for a request without a token while tokens are required.
pub const NO_TOKEN: &str =
    "This Meshwork requires an actor token: send `Authorization: Bearer mw_…`; \
     issue one with `meshwork-token create <actor>`";

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

/// Longest token lifetime that can be asked for, in days (ten years).
pub const MAX_TOKEN_DAYS: u32 = 3650;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Issues a token for the actor given by ID or name, valid until revoked. Returns the token, to
/// be shown once, and its ID for revoking.
pub async fn issue_token(
    graph: &mut AsyncGraph,
    actor: &str,
    label: &str,
) -> Result<(String, String, Actor)> {
    issue_expiring_token(graph, actor, label, None).await
}

/// Like [`issue_token`], valid for `expires_in_days` (1 to [`MAX_TOKEN_DAYS`]) when given.
pub async fn issue_expiring_token(
    graph: &mut AsyncGraph,
    actor: &str,
    label: &str,
    expires_in_days: Option<u32>,
) -> Result<(String, String, Actor)> {
    if let Some(days) = expires_in_days {
        if days == 0 || days > MAX_TOKEN_DAYS {
            return Err(AppError::BadRequest(format!(
                "Token lifetime must be 1 to {} days",
                MAX_TOKEN_DAYS
            )));
        }
    }
    let actor_id = db::resolve_actors(graph, &[actor.to_string()])
        .await?
        .remove(0);
    let token = generate_token();
    let token_id = Uuid::new_v4().to_string();
    let ttl_ms = expires_in_days.map(|days| i64::from(days) * DAY_MS);
    db::create_token(
        graph,
        &token_id,
        &hash_token(&token),
        &actor_id,
        label,
        ttl_ms,
    )
    .await?;
    let actor = db::get_actor(graph, &actor_id).await?;
    Ok((token, token_id, actor))
}

/// The actor given by ID or name, created with `kind` when there is none. Returns the actor and
/// whether it was created. For the `meshwork-token` CLI: with tokens required, the first actor
/// of a fresh install cannot be created over HTTP.
pub async fn ensure_actor(
    graph: &mut AsyncGraph,
    actor: &str,
    kind: ActorKind,
) -> Result<(Actor, bool)> {
    match db::resolve_actors(graph, &[actor.to_string()]).await {
        Ok(mut ids) => Ok((db::get_actor(graph, &ids.remove(0)).await?, false)),
        Err(AppError::NotFound(_)) => {
            let name = actor.trim();
            if name.is_empty() {
                return Err(AppError::BadRequest(
                    "Actor name cannot be empty".to_string(),
                ));
            }
            let id = Uuid::new_v4().to_string();
            db::ensure_actor_name_free(graph, name, &id).await?;
            db::create_actor(graph, &id, name, kind).await?;
            Ok((db::get_actor(graph, &id).await?, true))
        }
        Err(e) => Err(e),
    }
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

/// The token of an `Authorization: Bearer mw_…` header value, if it looks like one.
pub fn bearer_token(header: Option<&str>) -> Option<&str> {
    header?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| t.starts_with(TOKEN_PREFIX))
}

/// The actor behind an `Authorization` header value. Without a header: `None` if anonymous
/// requests are allowed, else 401. Anything but a known, unexpired `Bearer` token is a 401.
pub async fn authenticate(
    graph: &mut AsyncGraph,
    header: Option<&str>,
    allow_anonymous: bool,
) -> Result<Option<Actor>> {
    let Some(value) = header else {
        if allow_anonymous {
            return Ok(None);
        }
        return Err(AppError::Unauthorized(NO_TOKEN.to_string()));
    };
    let token =
        bearer_token(Some(value)).ok_or_else(|| AppError::Unauthorized(NOT_A_TOKEN.to_string()))?;
    match db::actor_by_token_hash(graph, &hash_token(token)).await? {
        Some(actor) => Ok(Some(actor)),
        None => Err(AppError::Unauthorized(
            "Unknown, revoked or expired token; issue a new one with `meshwork-token create <actor>`"
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
        match authenticate(&mut graph, header, state.allow_anonymous).await {
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
