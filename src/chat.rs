//! `POST /v1/decisions/chat`: decides whether a chat message may proceed.
//!
//! The message content is inspected in memory and discarded. It is never
//! echoed in the response, stored, or logged; only byte offsets are reported.

use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    detectors::{Detector, Finding},
    AppState,
};

/// Request body limit for this route only (the global limit stays at 4 KiB).
pub(crate) const MAX_BODY_BYTES: usize = 32 * 1024;

/// Maximum size of `content`, in UTF-8 bytes.
pub(crate) const MAX_CONTENT_BYTES: usize = 16 * 1024;

const ACTION_ALLOW: &str = "allow";
const ACTION_BLOCK: &str = "block";

// No `Debug` on purpose: a `{:?}` of this struct would print the content.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChatDecisionRequest {
    // Validated on parse so the contract is enforced now; no detector reads
    // it yet, which is why the compiler would otherwise flag it as unused.
    #[allow(dead_code)]
    role: Role,
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    System,
    Assistant,
}

#[derive(Serialize)]
pub(crate) struct ChatDecisionResponse {
    action: &'static str,
    reason_codes: Vec<&'static str>,
    findings: Vec<Finding>,
}

impl ChatDecisionResponse {
    fn from_findings(findings: Vec<Finding>) -> Self {
        let mut reason_codes: Vec<&'static str> = Vec::new();
        for finding in &findings {
            if !reason_codes.contains(&finding.code) {
                reason_codes.push(finding.code);
            }
        }
        let action = if findings.is_empty() {
            ACTION_ALLOW
        } else {
            ACTION_BLOCK
        };
        Self {
            action,
            reason_codes,
            findings,
        }
    }
}

pub(crate) async fn decide_chat(
    State(state): State<AppState>,
    input: Result<Json<ChatDecisionRequest>, JsonRejection>,
) -> Result<Json<ChatDecisionResponse>, StatusCode> {
    let Json(input) = input.map_err(rejection_status)?;
    if input.content.len() > MAX_CONTENT_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let findings = scan(state.detectors.as_slice(), &input.content);
    Ok(Json(ChatDecisionResponse::from_findings(findings)))
}

/// Maps axum's JSON rejections onto this route's contract.
///
/// axum answers 422 for a well-formed document with the wrong shape (unknown
/// field, invalid role, missing field). This API treats every contract
/// violation as 400. Other rejections keep their status: 413 for a body over
/// the limit and 415 for a missing JSON content type.
fn rejection_status(rejection: JsonRejection) -> StatusCode {
    match rejection {
        JsonRejection::JsonDataError(_) | JsonRejection::JsonSyntaxError(_) => {
            StatusCode::BAD_REQUEST
        }
        other => other.status(),
    }
}

fn scan(detectors: &[Box<dyn Detector>], content: &str) -> Vec<Finding> {
    detectors
        .iter()
        .flat_map(|detector| detector.scan(content))
        .collect()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::{MAX_BODY_BYTES, MAX_CONTENT_BYTES};
    use crate::{app, AppState};

    const REASON: &str = "content.invisible_unicode";

    fn test_key() -> String {
        "a".repeat(32)
    }

    fn test_app() -> axum::Router {
        app(AppState::new(test_key(), vec![]).expect("valid test config"))
    }

    async fn chat(body: String, key: Option<&str>) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/v1/decisions/chat")
            .header("content-type", "application/json");
        if let Some(key) = key {
            request = request.header("x-voidgate-key", key);
        }
        let request = request.body(Body::from(body)).expect("valid request");
        let response = test_app().oneshot(request).await.expect("router response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body");
        (
            status,
            String::from_utf8(body.to_vec()).expect("utf8 response"),
        )
    }

    fn message(content: &str) -> String {
        json!({ "role": "user", "content": content }).to_string()
    }

    async fn decide(content: &str) -> (StatusCode, Value) {
        let (status, body) = chat(message(content), Some(&test_key())).await;
        let json: Value = serde_json::from_str(&body).expect("json response");
        (status, json)
    }

    fn span(finding: &Value) -> (Option<u64>, Option<u64>) {
        (finding["start"].as_u64(), finding["end"].as_u64())
    }

    #[tokio::test]
    async fn allows_clean_text_in_portuguese_and_english() {
        for content in [
            "Olá! Pode me explicar como funciona a autenticação?",
            "Hello! Could you summarize this paragraph for me?",
            "",
        ] {
            let (status, json) = decide(content).await;
            assert_eq!(status, StatusCode::OK, "content {content:?}");
            assert_eq!(
                json,
                json!({ "action": "allow", "reason_codes": [], "findings": [] })
            );
        }
    }

    #[tokio::test]
    async fn blocks_right_to_left_override_with_byte_offsets() {
        // "Olá " is 5 bytes (á = 2 bytes); U+202E is 3 bytes.
        let (status, json) = decide("Olá \u{202E}mundo").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json,
            json!({
                "action": "block",
                "reason_codes": [REASON],
                "findings": [
                    { "code": REASON, "detector": "invisible_unicode", "start": 5, "end": 8 }
                ]
            })
        );
    }

    #[tokio::test]
    async fn blocks_zero_width_space_and_lists_reason_code_once() {
        let (status, json) = decide("a\u{200B}b\u{200B}c").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["action"], "block");
        assert_eq!(json["reason_codes"], json!([REASON]));
        let findings = json["findings"].as_array().expect("findings array");
        assert_eq!(findings.len(), 2);
        assert_eq!(span(&findings[0]), (Some(1), Some(4)));
        assert_eq!(span(&findings[1]), (Some(5), Some(8)));
    }

    #[tokio::test]
    async fn authentication_precedes_json_parsing() {
        let invalid_json = "not json".to_owned();
        assert_eq!(
            chat(invalid_json.clone(), None).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            chat(invalid_json, Some(&"b".repeat(32))).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn rejects_contract_violations_with_bad_request() {
        for body in [
            json!({ "role": "admin", "content": "hi" }),
            json!({ "role": "user", "content": "hi", "extra": 1 }),
            json!({ "role": "user" }),
            json!({ "role": "user", "content": 42 }),
            json!([]),
        ] {
            let (status, _) = chat(body.to_string(), Some(&test_key())).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "body {body}");
        }
        let (status, _) = chat("{not json".to_owned(), Some(&test_key())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_content_above_limit_and_accepts_content_at_limit() {
        let at_limit = "a".repeat(MAX_CONTENT_BYTES);
        assert_eq!(decide(&at_limit).await.0, StatusCode::OK);

        let above_limit = "a".repeat(MAX_CONTENT_BYTES + 1);
        let (status, _) = chat(message(&above_limit), Some(&test_key())).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn route_body_limit_overrides_global_limit() {
        // Larger than the global 4 KiB limit, within this route's 32 KiB.
        let (status, _) = chat(message(&"a".repeat(5000)), Some(&test_key())).await;
        assert_eq!(status, StatusCode::OK);

        let oversized = "x".repeat(MAX_BODY_BYTES + 1);
        let (status, _) = chat(oversized, Some(&test_key())).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn response_never_echoes_content() {
        let secret = "SEGREDO-f3a9c1";
        for content in [secret.to_owned(), format!("{secret}\u{200B}")] {
            let (status, body) = chat(message(&content), Some(&test_key())).await;
            assert_eq!(status, StatusCode::OK);
            assert!(!body.contains(secret), "response leaked content: {body}");
            assert!(
                !body.contains('\u{200B}'),
                "response leaked content: {body}"
            );
        }
    }
}
