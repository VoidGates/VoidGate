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
mod metrics;
mod models;
mod rate_limit;
mod runtime;
mod server;
mod services;
mod state;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};

use handlers::{chat, health, http};

#[cfg(debug_assertions)]
pub use handlers::docs::dev_docs;
pub use rate_limit::RateLimitConfig;
pub use server::serve;
pub use state::AppState;

const MAX_BODY_BYTES: usize = 4096;

pub fn app(state: AppState) -> Router {
    // Decision routes, innermost first: the concurrency/timeout guard sits
    // closest to the handler, the limiter runs once the caller is known.
    let decisions = Router::new()
        .route("/v1/decisions/http", post(http::decide_http))
        .route(
            "/v1/decisions/chat",
            post(chat::decide_chat).layer(DefaultBodyLimit::max(chat::MAX_BODY_BYTES)),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            runtime::protect,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::enforce,
        ));

    // Scrapes are authenticated but do not consume decision capacity.
    let metrics = Router::new().route("/metrics", get(metrics::export));

    // `layer` (not `route_layer`) so the fallback is authenticated too: without
    // a valid key, an unknown route answers exactly like a real one.
    let protected = Router::new()
        .merge(decisions)
        .merge(metrics)
        .fallback(error::not_found)
        .method_not_allowed_fallback(error::method_not_allowed)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::authorize,
        ));

    Router::new()
        .route("/health", get(health::live))
        .route("/live", get(health::live))
        .route("/ready", get(health::ready))
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

    use super::{app, error::AppError, AppState, RateLimitConfig};

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

    #[tokio::test]
    async fn limits_authenticated_decisions_without_affecting_health() {
        let state = AppState::with_rate_limit(
            "a".repeat(32),
            vec![],
            RateLimitConfig::per_minute(2).expect("valid limit"),
        )
        .expect("valid test config");
        let router = app(state);
        let request = |key: Option<&str>| {
            let mut builder = Request::builder()
                .method("POST")
                .uri("/v1/decisions/http")
                .header("content-type", "application/json");
            if let Some(key) = key {
                builder = builder.header("x-voidgate-key", key);
            }
            builder
                .body(Body::from(r#"{"method":"GET","path":"/public"}"#))
                .expect("request")
        };

        assert_eq!(
            router
                .clone()
                .oneshot(request(None))
                .await
                .expect("response")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let key = "a".repeat(32);
        for _ in 0..2 {
            assert_eq!(
                router
                    .clone()
                    .oneshot(request(Some(&key)))
                    .await
                    .expect("response")
                    .status(),
                StatusCode::OK
            );
        }
        let limited = router
            .clone()
            .oneshot(request(Some(&key)))
            .await
            .expect("response");
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited.headers().contains_key("retry-after"));
        let body = to_bytes(limited.into_body(), 4096).await.expect("body");
        assert_eq!(
            body,
            r#"{"error":{"code":"rate_limited","message":"Too many requests."}}"#.as_bytes()
        );

        let health = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(health.status(), StatusCode::OK);
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
    async fn without_key_no_request_shape_changes_the_answer() {
        let wrong = "b".repeat(32);
        for (uri, key, content_type, body) in [
            (
                "/v1/decisions/chat",
                None,
                "application/json",
                "{}".to_owned(),
            ),
            (
                "/v1/decisions/chat",
                Some(&wrong),
                "text/plain",
                "x".to_owned(),
            ),
            ("/v1/decisions/http", None, "text/plain", "x".to_owned()),
            (
                "/v1/decisions/http",
                None,
                "application/json",
                "x".repeat(200_000),
            ),
            (
                "/v1/decisions/nope",
                Some(&wrong),
                "application/json",
                "{}".to_owned(),
            ),
        ] {
            let mut request = Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", content_type);
            if let Some(key) = key {
                request = request.header("x-voidgate-key", key);
            }
            let response = test_app()
                .oneshot(request.body(Body::from(body)).expect("request"))
                .await
                .expect("response");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
            assert!(response.headers().get("allow").is_none(), "{uri}");
            let body = to_bytes(response.into_body(), 4096).await.expect("body");
            assert_eq!(body, UNAUTHORIZED.as_bytes(), "{uri}");
        }
    }

    #[tokio::test]
    async fn health_routes_answer_ok_and_reject_other_methods_in_the_error_format() {
        for uri in ["/health", "/live"] {
            let (status, body) = send(test_app(), "GET", uri, false).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert_eq!(body, r#"{"status":"ok"}"#, "{uri}");
            let (status, body) = send(test_app(), "POST", uri, false).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
            assert!(body.contains(r#""code":"method_not_allowed""#), "{uri}");
        }

        let (status, body) = send(test_app(), "GET", "/ready", false).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"status":"ready"}"#);
        let (status, body) = send(test_app(), "POST", "/ready", false).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert!(body.contains(r#""code":"method_not_allowed""#));
    }

    #[tokio::test]
    async fn metrics_is_authenticated_and_does_not_spend_decision_capacity() {
        let state = AppState::with_rate_limit(
            "a".repeat(32),
            vec![],
            RateLimitConfig::per_minute(1).expect("valid limit"),
        )
        .expect("valid test config");
        let router = app(state);

        let (status, _) = send(router.clone(), "GET", "/metrics", false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, body) = send(router.clone(), "GET", "/metrics", true).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("voidgate_authenticated_decisions_total"));
        assert!(
            !body.contains("a".repeat(32).as_str()),
            "metrics leak the key"
        );

        let (status, _) = send(router.clone(), "POST", "/v1/decisions/http", true).await;
        assert_eq!(
            status,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "the scrape must not consume the single allowed decision"
        );
    }

    #[tokio::test]
    async fn rejects_duplicate_json_keys_and_missing_content_type() {
        let key = "a".repeat(32);
        let (status, body) = decision(
            r#"{"method":"GET","path":"/a","path":"/internal/backup"}"#,
            Some(&key),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body, INVALID_BODY);

        let request = Request::builder()
            .method("POST")
            .uri("/v1/decisions/http")
            .header("x-voidgate-key", &key)
            .body(Body::from(r#"{"method":"GET","path":"/a"}"#))
            .expect("request");
        let response = test_app().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[test]
    fn key_must_be_32_to_128_bytes_that_a_header_can_carry() {
        let new = |key: String| AppState::new(key, vec![]);
        assert!(new("a".repeat(31)).is_err());
        assert!(new("a".repeat(32)).is_ok());
        assert!(new("a".repeat(128)).is_ok());
        assert!(new("a".repeat(129)).is_err());
        assert!(new(format!("{} ", "a".repeat(32))).is_err());
        assert!(new(format!(" {}", "a".repeat(32))).is_err());
        assert!(new(format!("{}\n", "a".repeat(32))).is_err());
        assert!(new(format!("{}\u{0007}", "a".repeat(32))).is_err());
    }

    #[test]
    fn spec_lists_exactly_the_error_codes_and_statuses_the_code_can_return() {
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("docs/openapi.json")).expect("valid openapi json");
        let mut documented: Vec<&str> = spec["components"]["schemas"]["Error"]["properties"]
            ["error"]["properties"]["code"]["enum"]
            .as_array()
            .expect("error code enum")
            .iter()
            .filter_map(|code| code.as_str())
            .collect();
        let mut actual: Vec<&str> = AppError::ALL.iter().map(|error| error.code()).collect();
        documented.sort_unstable();
        actual.sort_unstable();
        assert_eq!(documented, actual);

        for path in [
            "/health",
            "/live",
            "/ready",
            "/metrics",
            "/v1/decisions/http",
            "/v1/decisions/chat",
        ] {
            assert!(spec["paths"][path].is_object(), "spec is missing {path}");
        }
        for path in ["/v1/decisions/http", "/v1/decisions/chat"] {
            let responses = &spec["paths"][path]["post"]["responses"];
            for status in [
                "200", "400", "401", "404", "405", "413", "415", "429", "503", "504",
            ] {
                assert!(responses[status].is_object(), "{path} is missing {status}");
            }
        }
    }

    #[cfg(debug_assertions)]
    mod dev_docs {
        use axum::{
            body::{to_bytes, Body},
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;

        use super::{test_app, UNAUTHORIZED};
        use crate::dev_docs;

        async fn docs(method: &str, uri: &str, host: &str) -> (StatusCode, Option<String>, String) {
            let request = Request::builder()
                .method(method)
                .uri(uri)
                .header("host", host)
                .body(Body::empty())
                .expect("request");
            let response = test_app()
                .merge(dev_docs())
                .oneshot(request)
                .await
                .expect("response");
            let status = response.status();
            let csp = response
                .headers()
                .get("content-security-policy")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let body = to_bytes(response.into_body(), 64 * 1024)
                .await
                .expect("body");
            (status, csp, String::from_utf8(body.to_vec()).expect("utf8"))
        }

        #[tokio::test]
        async fn serves_ui_with_csp_and_a_valid_spec_to_local_callers() {
            let (status, csp, body) = docs("GET", "/docs", "127.0.0.1:8080").await;
            assert_eq!(status, StatusCode::OK);
            assert!(body.contains("swagger-ui"));
            let csp = csp.expect("content-security-policy header");
            assert!(csp.contains("default-src 'none'") && csp.contains("frame-ancestors 'none'"));

            let (status, _, body) = docs("GET", "/docs/openapi.json", "localhost:8080").await;
            assert_eq!(status, StatusCode::OK);
            assert!(serde_json::from_str::<serde_json::Value>(&body).is_ok());
        }

        #[tokio::test]
        async fn answers_like_a_missing_route_when_the_host_is_not_this_machine() {
            for host in ["api.example.com", "attacker.example:8080", "10.0.0.5:8080"] {
                for uri in ["/docs", "/docs/openapi.json"] {
                    let (status, _, body) = docs("GET", uri, host).await;
                    assert_eq!(status, StatusCode::UNAUTHORIZED, "{host} {uri}");
                    assert_eq!(body, UNAUTHORIZED, "{host} {uri}");
                }
            }
        }

        #[tokio::test]
        async fn other_methods_use_the_error_format() {
            let (status, _, body) = docs("POST", "/docs", "127.0.0.1:8080").await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
            assert!(body.contains(r#""code":"method_not_allowed""#));
        }
    }
}
