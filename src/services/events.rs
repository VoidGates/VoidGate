//! Best-effort recording of decision events.
//!
//! The decision is the product; the event log is a side effect. A failure to
//! record must never change what the caller receives, so it is counted and
//! then forgotten.

use std::sync::atomic::Ordering;

use crate::{
    events::{Direction, EventLog, Route},
    metrics::Metrics,
};

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
