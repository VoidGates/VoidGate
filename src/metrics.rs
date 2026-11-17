use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
};

use crate::AppState;

/// Aggregate counters only: no request bodies, credentials, paths, or client labels.
#[derive(Default)]
pub(crate) struct Metrics {
    pub(crate) authenticated_decisions: AtomicU64,
    pub(crate) auth_rejections: AtomicU64,
    pub(crate) rate_limit_rejections: AtomicU64,
    pub(crate) capacity_rejections: AtomicU64,
    pub(crate) decision_timeouts: AtomicU64,
}

pub(crate) async fn export(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = &state.metrics;
    let body = format!(
        "# HELP voidgate_authenticated_decisions_total Authenticated decision requests evaluated by the limiter.\n# TYPE voidgate_authenticated_decisions_total counter\nvoidgate_authenticated_decisions_total {}\n# HELP voidgate_auth_rejections_total Requests rejected by authentication.\n# TYPE voidgate_auth_rejections_total counter\nvoidgate_auth_rejections_total {}\n# HELP voidgate_rate_limit_rejections_total Requests rejected by the rate limiter.\n# TYPE voidgate_rate_limit_rejections_total counter\nvoidgate_rate_limit_rejections_total {}\n# HELP voidgate_capacity_rejections_total Requests rejected because decision capacity is full.\n# TYPE voidgate_capacity_rejections_total counter\nvoidgate_capacity_rejections_total {}\n# HELP voidgate_decision_timeouts_total Decision requests that exceeded the processing timeout.\n# TYPE voidgate_decision_timeouts_total counter\nvoidgate_decision_timeouts_total {}\n",
        metrics.authenticated_decisions.load(Ordering::Relaxed),
        metrics.auth_rejections.load(Ordering::Relaxed),
        metrics.rate_limit_rejections.load(Ordering::Relaxed),
        metrics.capacity_rejections.load(Ordering::Relaxed),
        metrics.decision_timeouts.load(Ordering::Relaxed),
    );
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
