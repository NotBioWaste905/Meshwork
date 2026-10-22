//! Bearer-token authentication for the MCP endpoint. Tokens are actor tokens issued by the
//! backend's `meshwork-token` CLI; this server keeps none of its own. Each request's token is
//! checked against the backend (`GET /v0/me`) and then forwarded on every backend call the tools
//! make, so the backend sees which actor acts.

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rmcp::{
    ErrorData,
    handler::server::common::{AsRequestContext, FromContextPart},
};

use crate::{backend::Backend, model::Actor};

/// Who calls a tool: a backend client acting as them, and their actor. Put into the HTTP request
/// by [`require_actor`]; tools take it as an argument.
#[derive(Clone)]
pub struct Caller {
    pub backend: Backend,
    pub actor: Actor,
}

impl<C: AsRequestContext> FromContextPart<C> for Caller {
    fn from_context_part(context: &mut C) -> Result<Self, ErrorData> {
        context
            .as_request_context()
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<Caller>())
            .cloned()
            .ok_or_else(|| ErrorData::internal_error("request has no authenticated caller", None))
    }
}

/// Every actor token starts with this; checked here so a wrong value fails with a clear message.
const TOKEN_PREFIX: &str = "mw_";

/// The token of an `Authorization: Bearer <token>` header value.
pub fn bearer(header_value: Option<&str>) -> Option<&str> {
    header_value
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

fn unauthorized(message: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        message,
    )
        .into_response()
}

pub async fn require_actor(
    State(backend): State<Backend>,
    mut req: Request,
    next: Next,
) -> Response {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let Some(token) = bearer(presented) else {
        return unauthorized("missing bearer token: send 'Authorization: Bearer mw_…'");
    };
    if !token.starts_with(TOKEN_PREFIX) {
        return unauthorized(
            "not a Meshwork token: tokens start with 'mw_'. A token id (a UUID, as \
             `meshwork-token list` shows) is not the token; issue a new one with \
             `meshwork-token create <actor>`",
        );
    }
    let backend = backend.with_token(token);
    match backend.me().await {
        Ok(Some(actor)) => {
            req.extensions_mut().insert(Caller { backend, actor });
            next.run(req).await
        }
        Ok(None) => unauthorized(
            "unknown or revoked token; issue a new one with `meshwork-token create <actor>`",
        ),
        Err(e) => {
            tracing::warn!("cannot check token: {e}");
            (StatusCode::BAD_GATEWAY, "cannot reach the Meshwork backend").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bearer_tokens() {
        assert_eq!(bearer(Some("Bearer mw_abc")), Some("mw_abc"));
        assert_eq!(bearer(Some("Bearer  mw_abc ")), Some("mw_abc"));
        assert_eq!(bearer(Some("mw_abc")), None);
        assert_eq!(bearer(Some("Bearer ")), None);
        assert_eq!(bearer(None), None);
    }
}
