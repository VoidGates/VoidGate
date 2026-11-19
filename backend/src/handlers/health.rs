//! Public liveness and readiness routes.

use axum::{extract::State, http::StatusCode, Json};

use crate::{models::HealthResponse, AppState};

/// Process is up; it says nothing about whether it can decide.
pub(crate) async fn live() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Ready to accept decisions: the in-memory limiter must still be usable.
pub(crate) async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    if state.rate_limiter.is_available() {
        (StatusCode::OK, Json(HealthResponse { status: "ready" }))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "not_ready",
            }),
        )
    }
}
