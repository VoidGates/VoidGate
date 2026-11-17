use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

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
    pub(crate) decision_errors: AtomicU64,
    pub(crate) decision_duration_micros_total: AtomicU64,
    pub(crate) decision_duration_samples: AtomicU64,
}

impl Metrics {
    pub(crate) fn observe_decision_duration(&self, elapsed: Duration) {
        self.decision_duration_micros_total
            .fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
        self.decision_duration_samples
            .fetch_add(1, Ordering::Relaxed);
    }
}

pub(crate) async fn export(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = &state.metrics;
    let body = format!(
        "# HELP voidgate_authenticated_decisions_total Authenticated decision requests evaluated by the limiter.\n# TYPE voidgate_authenticated_decisions_total counter\nvoidgate_authenticated_decisions_total {}\n# HELP voidgate_auth_rejections_total Requests rejected by authentication.\n# TYPE voidgate_auth_rejections_total counter\nvoidgate_auth_rejections_total {}\n# HELP voidgate_rate_limit_rejections_total Requests rejected by the rate limiter.\n# TYPE voidgate_rate_limit_rejections_total counter\nvoidgate_rate_limit_rejections_total {}\n# HELP voidgate_capacity_rejections_total Requests rejected because decision capacity is full.\n# TYPE voidgate_capacity_rejections_total counter\nvoidgate_capacity_rejections_total {}\n# HELP voidgate_decision_timeouts_total Decision requests that exceeded the processing timeout.\n# TYPE voidgate_decision_timeouts_total counter\nvoidgate_decision_timeouts_total {}\n# HELP voidgate_decision_errors_total Decision responses with a non-success status.\n# TYPE voidgate_decision_errors_total counter\nvoidgate_decision_errors_total {}\n# HELP voidgate_decision_duration_seconds Decision handler duration summary.\n# TYPE voidgate_decision_duration_seconds summary\nvoidgate_decision_duration_seconds_sum {:.6}\nvoidgate_decision_duration_seconds_count {}\n",
        metrics.authenticated_decisions.load(Ordering::Relaxed),
        metrics.auth_rejections.load(Ordering::Relaxed),
        metrics.rate_limit_rejections.load(Ordering::Relaxed),
        metrics.capacity_rejections.load(Ordering::Relaxed),
        metrics.decision_timeouts.load(Ordering::Relaxed),
        metrics.decision_errors.load(Ordering::Relaxed),
        metrics.decision_duration_micros_total.load(Ordering::Relaxed) as f64 / 1_000_000.0,
        metrics.decision_duration_samples.load(Ordering::Relaxed),
    );
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}
