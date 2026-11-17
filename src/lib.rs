mod auth;
mod chat;
mod detectors;
mod metrics;
mod policy;
mod rate_limit;
mod runtime;

use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use auth::{authorize, ApiKeyVerifier};
use detectors::Detector;
use metrics::Metrics;
use policy::{valid_path, Decision, ExactPathPolicy};
pub use rate_limit::RateLimitConfig;
use rate_limit::RateLimiter;

const MAX_BODY_BYTES: usize = 4096;
const MAX_CONCURRENT_DECISIONS: usize = 256;

#[derive(Clone)]
pub struct AppState {
    key_verifier: ApiKeyVerifier,
    policy: Arc<ExactPathPolicy>,
    detectors: Arc<Vec<Box<dyn Detector>>>,
    rate_limiter: Arc<RateLimiter>,
    decision_slots: Arc<Semaphore>,
    metrics: Arc<Metrics>,
}

impl AppState {
    pub fn new(api_key: String, decoy_paths: Vec<String>) -> Result<Self, String> {
        Self::with_rate_limit(api_key, decoy_paths, RateLimitConfig::default())
    }

    pub fn with_rate_limit(
        api_key: String,
        decoy_paths: Vec<String>,
        rate_limit: RateLimitConfig,
    ) -> Result<Self, String> {
        Ok(Self {
            key_verifier: ApiKeyVerifier::new(&api_key)?,
            policy: Arc::new(ExactPathPolicy::new(decoy_paths)?),
            detectors: Arc::new(detectors::default_detectors()),
            rate_limiter: Arc::new(RateLimiter::new(rate_limit)),
            decision_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_DECISIONS)),
            metrics: Arc::new(Metrics::default()),
        })
    }
}

pub fn app(state: AppState) -> Router {
    let decisions = Router::new()
        .route("/v1/decisions/http", post(decide_http))
        .route(
            "/v1/decisions/chat",
            post(chat::decide_chat).layer(DefaultBodyLimit::max(chat::MAX_BODY_BYTES)),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), runtime::protect))
        .route_layer(middleware::from_fn_with_state(state.clone(), rate_limit::enforce))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));

    let metrics = Router::new()
        .route("/metrics", get(metrics::export))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));

    Router::new()
        .route("/health", get(live))
        .route("/live", get(live))
        .route("/ready", get(ready))
        .merge(metrics)
        .merge(decisions)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    if state.rate_limiter.is_available() {
        (StatusCode::OK, Json(HealthResponse { status: "ready" }))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse { status: "not_ready" }),
        )
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HttpDecisionRequest {
    method: String,
    path: String,
}

#[derive(Serialize)]
struct HttpDecisionResponse {
    action: &'static str,
    reason_code: &'static str,
}

async fn decide_http(
    State(state): State<AppState>,
    Json(input): Json<HttpDecisionRequest>,
) -> Result<Json<HttpDecisionResponse>, StatusCode> {
    if !valid_method(&input.method) || !valid_path(&input.path) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (action, reason_code) = match state.policy.evaluate(&input.path) {
        Decision::Decoy => ("decoy", "registered_decoy_path"),
        Decision::Allow => ("allow", "no_matching_rule"),
    };

    Ok(Json(HttpDecisionResponse {
        action,
        reason_code,
    }))
}

fn valid_method(method: &str) -> bool {
    matches!(
        method,
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    )
}
#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::{app, AppState, RateLimitConfig};

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
            router.clone().oneshot(request(None)).await.expect("response").status(),
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
}
