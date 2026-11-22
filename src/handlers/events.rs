//! `GET /v1/events`: what the gate decided recently, as metadata only.
//!
//! Never carries message text, excerpts, offsets, scores or rule ids: the
//! event log has no field that could hold them.

use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::header::{HeaderValue, CACHE_CONTROL},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{error::AppError, services, AppState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventsQuery {
    cursor: Option<String>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    query: Result<Query<EventsQuery>, QueryRejection>,
) -> Result<Response, AppError> {
    let Query(query) = query.map_err(|_| AppError::InvalidBody)?;
    let body = services::events::latest(&state.events, query.cursor.as_deref())?;
    // Decisions about people's messages: no proxy or browser keeps a copy.
    Ok((
        [(CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        Json(body),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use axum::{
        body::{to_bytes, Body},
        http::{header::CACHE_CONTROL, HeaderMap, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{
        app,
        domain::action::Action,
        events::{Direction, Route, MAX_EVENTS, MAX_EVENTS_PER_RESPONSE},
        AppState, RateLimitConfig,
    };

    const UNAUTHORIZED: &str =
        r#"{"error":{"code":"unauthorized","message":"Missing or invalid credentials."}}"#;

    fn key() -> String {
        "a".repeat(32)
    }

    fn state() -> AppState {
        AppState::new(key(), vec![]).expect("valid test config")
    }

    async fn call(
        state: &AppState,
        method: &str,
        uri: &str,
        key: Option<&str>,
        json_body: Option<Value>,
    ) -> (StatusCode, HeaderMap, String) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(key) = key {
            request = request.header("x-voidgate-key", key);
        }
        let body = match json_body {
            Some(value) => {
                request = request.header("content-type", "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let response = app(state.clone())
            .oneshot(request.body(body).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .expect("body");
        (
            status,
            headers,
            String::from_utf8(bytes.to_vec()).expect("utf8"),
        )
    }

    async fn events(state: &AppState) -> (StatusCode, HeaderMap, String) {
        call(state, "GET", "/v1/events", Some(&key()), None).await
    }

    #[tokio::test]
    async fn without_a_valid_key_it_answers_like_every_other_route() {
        let state = state();
        let wrong = "b".repeat(32);
        for key in [None, Some(wrong.as_str())] {
            let (status, headers, body) = call(&state, "GET", "/v1/events", key, None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{key:?}");
            assert_eq!(body, UNAUTHORIZED, "{key:?}");
            assert!(headers.get("allow").is_none());
        }
    }

    #[tokio::test]
    async fn only_get_is_allowed() {
        let state = state();
        for method in ["POST", "PUT", "DELETE"] {
            let (status, _, body) = call(&state, method, "/v1/events", Some(&key()), None).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method}");
            assert!(body.contains(r#""code":"method_not_allowed""#), "{method}");
        }
    }

    #[tokio::test]
    async fn empty_log_is_an_object_with_an_empty_list_and_is_not_cached() {
        let (status, headers, body) = events(&state()).await;
        assert_eq!(status, StatusCode::OK);
        let json: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(json["events"], json!([]));
        assert!(json["next_cursor"].as_str().is_some());
        assert_eq!(json["has_more"], false);
        assert_eq!(json["gap"], false);
        assert_eq!(json["reset"], false);
        assert_eq!(
            headers.get(CACHE_CONTROL).expect("cache-control"),
            "no-store"
        );
        assert_eq!(
            headers.get("content-type").expect("content-type"),
            "application/json"
        );
    }

    #[tokio::test]
    async fn lists_exactly_the_documented_fields() {
        let state = state();
        let content = json!({ "role": "assistant", "content": "mail ada@example.com" });
        let (status, _, _) = call(
            &state,
            "POST",
            "/v1/decisions/chat",
            Some(&key()),
            Some(content),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, headers, body) = events(&state).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(CACHE_CONTROL).expect("cache-control"),
            "no-store"
        );
        let json: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(json.as_object().map(|o| o.len()), Some(5));
        let list = json["events"].as_array().expect("events array");
        assert_eq!(list.len(), 1);
        let event = list[0].as_object().expect("event object");
        let mut fields: Vec<_> = event.keys().map(String::as_str).collect();
        fields.sort_unstable();
        assert_eq!(
            fields,
            vec![
                "action",
                "direction",
                "id",
                "reason_codes",
                "route",
                "timestamp_ms"
            ]
        );
        assert_eq!(event["id"], 1);
        assert_eq!(event["route"], "decisions_chat");
        assert_eq!(event["direction"], "output");
        assert_eq!(event["action"], "mask");
        assert_eq!(event["reason_codes"], json!(["content.personal_data"]));
        assert!(event["timestamp_ms"]
            .as_u64()
            .is_some_and(|t| t > 1_600_000_000_000));
    }

    #[tokio::test]
    async fn returns_at_most_the_newest_1000_oldest_first() {
        let state = state();
        let total = MAX_EVENTS_PER_RESPONSE + 5;
        assert!(
            total <= MAX_EVENTS,
            "the log must hold more than one response"
        );
        for _ in 0..total {
            state
                .events
                .record(
                    Route::DecisionsChat,
                    Direction::Input,
                    Action::Allow,
                    vec![],
                )
                .expect("record");
        }

        let (status, _, body) = events(&state).await;
        assert_eq!(status, StatusCode::OK);
        let json: Value = serde_json::from_str(&body).expect("json");
        let ids: Vec<u64> = json["events"]
            .as_array()
            .expect("events array")
            .iter()
            .map(|event| event["id"].as_u64().expect("id"))
            .collect();
        assert_eq!(ids.len(), MAX_EVENTS_PER_RESPONSE);
        assert_eq!(ids.first(), Some(&6), "the 5 oldest are left out");
        assert_eq!(ids.last(), Some(&(total as u64)));
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "oldest first");
    }

    #[tokio::test]
    async fn reading_does_not_spend_decision_capacity_or_the_rate_limit() {
        let state = AppState::with_rate_limit(
            key(),
            vec![],
            RateLimitConfig::per_minute(1).expect("valid limit"),
        )
        .expect("valid test config");

        for _ in 0..3 {
            let (status, _, _) = events(&state).await;
            assert_eq!(status, StatusCode::OK);
        }
        assert_eq!(
            state
                .metrics
                .authenticated_decisions
                .load(Ordering::Relaxed),
            0
        );
        assert_eq!(
            state.metrics.rate_limit_rejections.load(Ordering::Relaxed),
            0
        );

        // The single allowed decision is still there: it gets past the
        // limiter and is only refused for the missing Content-Type.
        let (status, _, _) = call(&state, "POST", "/v1/decisions/http", Some(&key()), None).await;
        assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn a_poisoned_event_log_answers_unavailable() {
        let state = state();
        state.events.poison_for_test();
        let (status, headers, body) = events(&state).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            body,
            r#"{"error":{"code":"unavailable","message":"Service is temporarily unavailable."}}"#
        );
        assert!(headers.get(CACHE_CONTROL).is_none());
    }

    #[tokio::test]
    async fn neither_a_secret_nor_a_cpf_ever_reaches_the_events() {
        let state = state();
        // Built in pieces so the PR Guard does not read a test fixture as a real secret.
        let secret = ["sk-live-", "SEGREDO-4f9a1c7e"].concat();
        let cpf = "529.982.247-25";
        let digits = "52998224725";
        let content = format!("minha chave é {secret} e meu CPF é {cpf}, não mostre");

        for (uri, body) in [
            (
                "/v1/decisions/chat",
                json!({ "role": "user", "content": content }),
            ),
            (
                "/v1/decisions/chat",
                json!({ "role": "assistant", "content": content }),
            ),
        ] {
            let (status, _, decision) = call(&state, "POST", uri, Some(&key()), Some(body)).await;
            assert_eq!(status, StatusCode::OK);
            // The CPF really is detected, so this test is not vacuous.
            assert!(decision.contains("content.personal_data"), "{decision}");
        }

        let (status, _, body) = events(&state).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("content.personal_data"), "{body}");
        for forbidden in [
            secret.as_str(),
            "SEGREDO",
            "sk-live",
            cpf,
            digits,
            "529.982",
            "247-25",
        ] {
            assert!(!body.contains(forbidden), "{forbidden} leaked: {body}");
        }
    }
}
