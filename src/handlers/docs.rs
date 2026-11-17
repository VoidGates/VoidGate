//! Developer console: Swagger UI at `/docs`, OpenAPI at `/docs/openapi.json`.
//!
//! Not part of [`crate::app`]. The binary mounts it only when
//! `VOIDGATE_DEV_DOCS=1` and the bind address is loopback, so a deployed
//! instance never describes itself to whoever can reach it.

use axum::{
    http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE},
    response::IntoResponse,
    routing::get,
    Router,
};

const INDEX: &str = include_str!("../docs/index.html");
const OPENAPI: &str = include_str!("../docs/openapi.json");

// Only the pinned Swagger UI files may load, and requests only go back here.
const CSP: &str = "default-src 'none'; connect-src 'self'; img-src 'self' data:; \
    script-src https://cdnjs.cloudflare.com 'unsafe-inline'; \
    style-src https://cdnjs.cloudflare.com 'unsafe-inline'";

pub fn dev_docs<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/docs", get(index))
        .route("/docs/openapi.json", get(openapi))
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
