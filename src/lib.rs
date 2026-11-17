//! VoidGate decision API.
//!
//! Layers, from the outside in:
//! `handlers` (HTTP) -> `services` (rules and orchestration) -> `domain`
//! (pure decisions). `models` holds the JSON bodies, `error` the single error
//! contract, `state` the configuration shared by requests.

mod auth;
mod domain;
mod error;
mod handlers;
mod models;
mod services;
mod state;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};

use handlers::{chat, health::health, http};

pub use handlers::docs::dev_docs;
pub use state::AppState;

const MAX_BODY_BYTES: usize = 4096;

pub fn app(state: AppState) -> Router {
    // `layer` (not `route_layer`) so the fallback is authenticated too: without
    // a valid key, an unknown route answers exactly like a real one.
    let protected = Router::new()
        .route("/v1/decisions/http", post(http::decide_http))
        .route(
            "/v1/decisions/chat",
            post(chat::decide_chat).layer(DefaultBodyLimit::max(chat::MAX_BODY_BYTES)),
        )
        .fallback(error::not_found)
        .method_not_allowed_fallback(error::method_not_allowed)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::authorize,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/live", get(health))
        .route("/ready", get(health))
        .method_not_allowed_fallback(error::method_not_allowed)
        .merge(protected)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::{app, dev_docs, AppState};

    fn test_app() -> axum::Router {
        app(
            AppState::new("a".repeat(32), vec!["/internal/backup".into()])
                .expect("valid test config"),
        )
    }

    async fn decision(body: &str, key: Option<&str>) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/v1/decisions/http")
            .header("content-type", "application/json");
        if let Some(key) = key {
            request = request.header("x-voidgate-key", key);
        }
        let request = request
            .body(Body::from(body.to_owned()))
            .expect("valid request");
        let response = test_app().oneshot(request).await.expect("router response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body");
        (
            status,
            String::from_utf8(body.to_vec()).expect("utf8 response"),
        )
    }

    #[tokio::test]
    async fn health_is_public() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn authentication_precedes_json_parsing() {
        assert_eq!(
            decision("invalid json", None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            decision("invalid json", Some(&"b".repeat(32))).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn exact_path_decides_and_invalid_path_is_rejected() {
        let key = "a".repeat(32);
        let (status, body) =
            decision(r#"{"method":"GET","path":"/internal/backup"}"#, Some(&key)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("\"action\":\"decoy\""));

        let (status, body) = decision(r#"{"method":"GET","path":"/public"}"#, Some(&key)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("\"action\":\"allow\""));

        for path in [
            "relative",
            "/internal/%62ackup",
            "/internal//backup",
            "/a/../backup",
        ] {
            let body = format!(r#"{{"method":"GET","path":"{path}"}}"#);
            assert_eq!(decision(&body, Some(&key)).await.0, StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn rejects_oversized_body() {
        let body = "x".repeat(4097);
        assert_eq!(
            decision(&body, Some(&"a".repeat(32))).await.0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[test]
    fn rejects_weak_key_and_invalid_decoy_path() {
        assert!(AppState::new("short".into(), vec![]).is_err());
        assert!(AppState::new("a".repeat(32), vec!["relative".into()]).is_err());
    }

    async fn send(
        router: axum::Router,
        method: &str,
        uri: &str,
        key: bool,
    ) -> (StatusCode, String) {
        let mut request = Request::builder().method(method).uri(uri);
        if key {
            request = request.header("x-voidgate-key", "a".repeat(32));
        }
        let response = router
            .oneshot(request.body(Body::empty()).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body");
        (
            status,
            String::from_utf8(body.to_vec()).expect("utf8 response"),
        )
    }

    const UNAUTHORIZED: &str =
        r#"{"error":{"code":"unauthorized","message":"Missing or invalid credentials."}}"#;
    const INVALID_BODY: &str = r#"{"error":{"code":"invalid_body","message":"Request body does not match the documented contract."}}"#;

    #[tokio::test]
    async fn without_key_every_non_health_route_answers_the_same() {
        for (method, uri) in [
            ("POST", "/v1/decisions/http"),
            ("GET", "/v1/decisions/http"),
            ("GET", "/v1/decisions/unknown"),
            ("GET", "/docs"),
            ("GET", "/"),
        ] {
            let (status, body) = send(test_app(), method, uri, false).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
            assert_eq!(body, UNAUTHORIZED, "{method} {uri}");
        }
    }

    #[tokio::test]
    async fn with_key_routing_errors_are_named() {
        let (status, body) = send(test_app(), "GET", "/nope", true).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains(r#""code":"not_found""#));

        let (status, body) = send(test_app(), "GET", "/v1/decisions/http", true).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert!(body.contains(r#""code":"method_not_allowed""#));
    }

    #[tokio::test]
    async fn body_errors_name_the_category_but_not_the_rule() {
        let key = "a".repeat(32);
        let (status, body) = decision("{not json", Some(&key)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains(r#""code":"invalid_json""#));

        for input in [
            r#"{"method":"GET"}"#,
            r#"{"method":"GET","path":"/a","extra":1}"#,
            r#"{"method":"TRACE","path":"/a"}"#,
            r#"{"method":"GET","path":"/a/%2e%2e/b"}"#,
        ] {
            let (status, body) = decision(input, Some(&key)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{input}");
            assert_eq!(body, INVALID_BODY, "{input}");
        }
    }

    #[tokio::test]
    async fn dev_docs_serve_ui_and_a_spec_covering_every_route() {
        let router = || test_app().merge(dev_docs());
        let (status, body) = send(router(), "GET", "/docs", false).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("swagger-ui"));

        let (status, body) = send(router(), "GET", "/docs/openapi.json", false).await;
        assert_eq!(status, StatusCode::OK);
        let spec: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");
        for path in [
            "/health",
            "/live",
            "/ready",
            "/v1/decisions/http",
            "/v1/decisions/chat",
        ] {
            assert!(spec["paths"][path].is_object(), "spec is missing {path}");
        }
    }
}
