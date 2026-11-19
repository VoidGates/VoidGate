//! Developer console: Swagger UI at `/docs`, OpenAPI at `/docs/openapi.json`.
//!
//! Compiled only into development builds (`debug_assertions`), so a release
//! binary cannot serve it whatever its configuration. In a development build
//! the binary mounts it only when `VOIDGATE_DEV_DOCS=1` and the bind address
//! is loopback, and it answers only requests addressed to this machine.

use axum::{
    extract::Request,
    http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE, HOST},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::error::{self, AppError};

const INDEX: &str = include_str!("../docs/index.html");
const OPENAPI: &str = include_str!("../docs/openapi.json");

// Only the pinned Swagger UI files may load, and requests only go back here.
const CSP: &str = "default-src 'none'; connect-src 'self'; img-src 'self' data:; \
    script-src https://cdnjs.cloudflare.com 'unsafe-inline'; \
    style-src https://cdnjs.cloudflare.com 'unsafe-inline'; \
    frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

pub fn dev_docs<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/docs", get(index))
        .route("/docs/openapi.json", get(openapi))
        .method_not_allowed_fallback(error::method_not_allowed)
        .layer(middleware::from_fn(local_host_only))
}

/// A loopback bind still receives requests relayed by a reverse proxy on the
/// same machine, or sent by a web page through DNS rebinding. Both carry a
/// `Host` that is not this machine, and get the answer of a route that does
/// not exist.
async fn local_host_only(request: Request, next: Next) -> Result<Response, AppError> {
    let host = request
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if is_local_host(host) {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Unauthorized)
    }
}

fn is_local_host(host: &str) -> bool {
    let name = match host.rsplit_once(':') {
        Some((name, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

async fn index() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/html; charset=utf-8"),
            (CONTENT_SECURITY_POLICY, CSP),
        ],
        INDEX,
    )
}

async fn openapi() -> impl IntoResponse {
    ([(CONTENT_TYPE, "application/json")], OPENAPI)
}

#[cfg(test)]
mod tests {
    use super::is_local_host;

    #[test]
    fn only_this_machine_counts_as_local() {
        for host in [
            "127.0.0.1",
            "127.0.0.1:8080",
            "localhost",
            "localhost:9090",
            "[::1]",
            "[::1]:8080",
        ] {
            assert!(is_local_host(host), "{host}");
        }
        for host in [
            "",
            "api.example.com",
            "api.example.com:8080",
            "localhost.evil.example",
            "evil.example:127.0.0.1",
            "127.0.0.1.evil.example",
            "127.0.0.1:",
            "0.0.0.0",
        ] {
            assert!(!is_local_host(host), "{host}");
        }
    }
}
