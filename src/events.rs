//! In-memory log of decision events, for `GET /v1/events`.
//!
//! Metadata only: which route, which direction, which action and which reason
//! codes. There is no field that could hold message text, an offset or a
//! score, so nothing sensitive can be stored by mistake. The log is local to
//! the process and is lost on restart.

use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use std::{sync::Arc, thread};

use serde::Serialize;

use crate::error::AppError;

/// Events kept at once; the oldest is dropped first.
pub(crate) const MAX_EVENTS: usize = 10_000;

/// Events returned by one `GET /v1/events`: the newest ones of the window.
pub(crate) const MAX_EVENTS_PER_RESPONSE: usize = 1_000;

/// How long an event stays visible, in milliseconds (30 minutes).
pub(crate) const WINDOW_MS: u64 = 30 * 60 * 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Route {
    DecisionsChat,
    ProxyChat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Event {
    /// Sequential, starting at 1. Never reused while the process runs.
    pub(crate) id: u64,
    /// Milliseconds since the Unix epoch.
    pub(crate) timestamp_ms: u64,
    pub(crate) route: Route,
    pub(crate) direction: Direction,
    /// `allow`, `mask` or `block`, as `services::chat::action_of` returns it.
    pub(crate) action: &'static str,
    pub(crate) reason_codes: Vec<&'static str>,
}

struct Inner {
    events: VecDeque<Event>,
    next_id: u64,
}

pub(crate) struct EventLog {
    inner: Mutex<Inner>,
}

impl EventLog {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                events: VecDeque::new(),
                next_id: 1,
            }),
        }
    }

    /// Appends one event, then drops what is older than the window or beyond
    /// the cap. `Unavailable` if the lock is poisoned or the clock is unusable.
    pub(crate) fn record(
        &self,
        route: Route,
        direction: Direction,
        action: &'static str,
        reason_codes: Vec<&'static str>,
    ) -> Result<(), AppError> {
        let mut inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        // Read the clock while holding the lock, so ids and timestamps grow
        // together even when two requests race (same reasoning as rate_limit).
        let now = now_ms()?;
        push_locked(&mut inner, now, route, direction, action, reason_codes);
        Ok(())
    }

    /// At most `limit` events of the window, oldest first.
    pub(crate) fn recent(&self, limit: usize) -> Result<Vec<Event>, AppError> {
        let inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        Ok(recent_locked(&inner, now_ms()?, limit))
    }

    /// Poisons the lock the way a panic in another thread would.
    #[cfg(test)]
    pub(crate) fn poison_for_test(self: &Arc<Self>) {
        let this = Arc::clone(self);
        let _ = thread::spawn(move || {
            let _guard = this.inner.lock().expect("lock");
            panic!("poison the event log");
        })
        .join();
    }

    #[cfg(test)]
    fn record_at(
        &self,
        now: u64,
        route: Route,
        direction: Direction,
        action: &'static str,
        reason_codes: Vec<&'static str>,
    ) -> Result<(), AppError> {
        let mut inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        push_locked(&mut inner, now, route, direction, action, reason_codes);
        Ok(())
    }

    #[cfg(test)]
    fn recent_at(&self, now: u64, limit: usize) -> Result<Vec<Event>, AppError> {
        let inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        Ok(recent_locked(&inner, now, limit))
    }
}

fn push_locked(
    inner: &mut Inner,
    now: u64,
    route: Route,
    direction: Direction,
    action: &'static str,
    reason_codes: Vec<&'static str>,
) {
    let id = inner.next_id;
    inner.next_id = id.saturating_add(1);
    inner.events.push_back(Event {
        id,
        timestamp_ms: now,
        route,
        direction,
        action,
        reason_codes,
    });

    let cutoff = now.saturating_sub(WINDOW_MS);
    while inner
        .events
        .front()
        .is_some_and(|event| event.timestamp_ms < cutoff)
    {
        inner.events.pop_front();
    }
    while inner.events.len() > MAX_EVENTS {
        inner.events.pop_front();
    }
}

fn recent_locked(inner: &Inner, now: u64, limit: usize) -> Vec<Event> {
    // Expired events can still sit in memory when no one has written for a
    // while, so reading filters by the window too.
    let cutoff = now.saturating_sub(WINDOW_MS);
    let mut newest_first: Vec<Event> = inner
        .events
        .iter()
        .rev()
        .filter(|event| event.timestamp_ms >= cutoff)
        .take(limit)
        .cloned()
        .collect();
    newest_first.reverse();
    newest_first
}

fn now_ms() -> Result<u64, AppError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Unavailable)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| AppError::Unavailable)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{Direction, Event, EventLog, Route, MAX_EVENTS, WINDOW_MS};
    use crate::error::AppError;

    const T0: u64 = 10_000_000;

    fn push(log: &EventLog, now: u64, action: &'static str) {
        log.record_at(now, Route::DecisionsChat, Direction::Input, action, vec![])
            .expect("record");
    }

    fn ids(events: &[Event]) -> Vec<u64> {
        events.iter().map(|event| event.id).collect()
    }

    #[test]
    fn returns_events_oldest_first_with_sequential_ids() {
        let log = EventLog::new();
        push(&log, T0, "allow");
        push(&log, T0 + 1, "mask");
        push(&log, T0 + 2, "block");

        let events = log.recent_at(T0 + 2, 10).expect("recent");
        assert_eq!(ids(&events), vec![1, 2, 3]);
        let actions: Vec<_> = events.iter().map(|event| event.action).collect();
        assert_eq!(actions, vec!["allow", "mask", "block"]);
    }

    #[test]
    fn limit_keeps_the_newest_and_still_returns_oldest_first() {
        let log = EventLog::new();
        for offset in 0..5 {
            push(&log, T0 + offset, "allow");
        }
        let events = log.recent_at(T0 + 5, 2).expect("recent");
        assert_eq!(ids(&events), vec![4, 5]);
        assert!(log.recent_at(T0 + 5, 0).expect("recent").is_empty());
    }

    #[test]
    fn cap_drops_the_oldest_events() {
        let log = EventLog::new();
        for _ in 0..MAX_EVENTS + 5 {
            push(&log, T0, "allow");
        }
        let events = log.recent_at(T0, usize::MAX).expect("recent");
        assert_eq!(events.len(), MAX_EVENTS);
        assert_eq!(events.first().map(|event| event.id), Some(6));
        assert_eq!(
            events.last().map(|event| event.id),
            Some(MAX_EVENTS as u64 + 5)
        );
    }

    #[test]
    fn writing_discards_events_older_than_the_window() {
        let log = EventLog::new();
        push(&log, T0, "allow");
        // Exactly 30 minutes old: still inside the window.
        push(&log, T0 + WINDOW_MS, "mask");
        assert_eq!(
            ids(&log.recent_at(T0 + WINDOW_MS, 10).expect("recent")),
            vec![1, 2]
        );
        // One millisecond more and the first one is gone for good.
        push(&log, T0 + WINDOW_MS + 1, "block");
        assert_eq!(
            ids(&log.recent_at(T0 + WINDOW_MS + 1, 10).expect("recent")),
            vec![2, 3]
        );
        assert_eq!(
            ids(&log.recent_at(T0, 10).expect("recent")),
            vec![2, 3],
            "the expired event was removed from memory, not only hidden"
        );
    }

    #[test]
    fn reading_hides_expired_events_even_without_new_writes() {
        let log = EventLog::new();
        push(&log, T0, "allow");
        assert!(log
            .recent_at(T0 + WINDOW_MS + 1, 10)
            .expect("recent")
            .is_empty());
    }

    #[test]
    fn a_clock_before_the_window_does_not_underflow() {
        let log = EventLog::new();
        push(&log, 5, "allow");
        assert_eq!(log.recent_at(5, 10).expect("recent").len(), 1);
    }

    #[test]
    fn recent_and_record_use_the_real_clock() {
        let log = EventLog::new();
        log.record(
            Route::ProxyChat,
            Direction::Output,
            "block",
            vec!["content.prompt_injection"],
        )
        .expect("record");
        let events = log.recent(10).expect("recent");
        assert_eq!(events.len(), 1);
        assert!(events[0].timestamp_ms > 1_600_000_000_000);
    }

    #[test]
    fn serializes_only_the_documented_fields() {
        let event = Event {
            id: 7,
            timestamp_ms: 1_700_000_000_000,
            route: Route::ProxyChat,
            direction: Direction::Output,
            action: "mask",
            reason_codes: vec!["content.personal_data"],
        };
        assert_eq!(
            serde_json::to_value(&event).expect("json"),
            json!({
                "id": 7,
                "timestamp_ms": 1_700_000_000_000_u64,
                "route": "proxy_chat",
                "direction": "output",
                "action": "mask",
                "reason_codes": ["content.personal_data"]
            })
        );
    }

    #[test]
    fn poisoned_lock_is_unavailable_for_writes_and_reads() {
        let log = Arc::new(EventLog::new());
        log.poison_for_test();

        assert_eq!(
            log.record(Route::DecisionsChat, Direction::Input, "allow", vec![]),
            Err(AppError::Unavailable)
        );
        assert_eq!(log.recent(10), Err(AppError::Unavailable));
    }
}
