use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::time::timeout;

use crate::{error::AppError, AppState};

const DECISION_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn protect(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let _permit = state
        .decision_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            state
                .metrics
                .capacity_rejections
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            AppError::Unavailable
        })?;

    let started = Instant::now();
    match timeout(DECISION_TIMEOUT, next.run(request)).await {
        Ok(response) => {
            state.metrics.observe_decision_duration(started.elapsed());
            if !response.status().is_success() {
                state
                    .metrics
                    .decision_errors
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(response)
        }
        Err(_) => {
            state.metrics.observe_decision_duration(started.elapsed());
            state
                .metrics
                .decision_timeouts
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            state
                .metrics
                .decision_errors
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(AppError::DecisionTimeout.into_response())
        }
    }
}
