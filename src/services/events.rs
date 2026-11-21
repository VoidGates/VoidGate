//! Best-effort recording of decision events.
//!
//! The decision is the product; the event log is a side effect. A failure to
//! record must never change what the caller receives, so it is counted and
//! then forgotten.

use std::sync::atomic::Ordering;

use crate::{
    error::AppError,
    events::{Direction, EventLog, Route, MAX_EVENTS_PER_RESPONSE},
    metrics::Metrics,
    models::events::EventsResponse,
};

/// The newest events of the window, oldest first. Unlike recording, reading
/// is the whole point of the call, so a broken log is an error (`503`).
pub(crate) fn latest(events: &EventLog) -> Result<EventsResponse, AppError> {
    Ok(EventsResponse {
        events: events.recent(MAX_EVENTS_PER_RESPONSE)?,
    })
}

pub(crate) fn record_best_effort(
    events: &EventLog,
    metrics: &Metrics,
    route: Route,
    direction: Direction,
    action: &'static str,
    reason_codes: Vec<&'static str>,
) {
    if events
        .record(route, direction, action, reason_codes)
        .is_err()
    {
        metrics.event_record_errors.fetch_add(1, Ordering::Relaxed);
    }
}
