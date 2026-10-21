//! Static bearer-token authentication for the MCP endpoint.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Comparison that does not bail out at the first differing byte.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn is_authorized(tokens: &[String], header_value: Option<&str>) -> bool {
    let Some(presented) = header_value.and_then(|v| v.strip_prefix("Bearer ")) else {
        return false;
    };
    // Check every token so timing does not reveal which one matched.
    tokens.iter().fold(false, |ok, t| {
        ct_eq(t.as_bytes(), presented.trim().as_bytes()) | ok
    })
}

pub async fn require_token(
    State(tokens): State<Arc<Vec<String>>>,
    req: Request,
    next: Next,
) -> Response {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if is_authorized(&tokens, presented) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            "missing or invalid bearer token",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_known_bearer_tokens() {
        let tokens = vec!["secret-a".to_string(), "secret-b".to_string()];
        assert!(is_authorized(&tokens, Some("Bearer secret-b")));
        assert!(!is_authorized(&tokens, Some("Bearer nope")));
        assert!(!is_authorized(&tokens, Some("secret-a")));
        assert!(!is_authorized(&tokens, None));
        assert!(!is_authorized(&[], Some("Bearer ")));
    }
}
