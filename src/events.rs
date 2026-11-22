//! In-memory log of decision events, for `GET /v1/events`.
//!
//! Metadata only: which route, which direction, which action and which reason
//! codes. There is no field that could hold message text, an offset or a
//! score, so nothing sensitive can be stored by mistake. The log is local to
//! the process and is lost on restart.

use std::{
    collections::{hash_map::RandomState, VecDeque},
    hash::BuildHasher,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use std::{sync::Arc, thread};

use serde::Serialize;

use crate::{domain::action::Action, error::AppError};

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
    pub(crate) action: Action,
    pub(crate) reason_codes: Vec<&'static str>,
}

struct Inner {
    events: VecDeque<Event>,
    next_id: u64,
}

pub(crate) struct EventLog {
    inner: Mutex<Inner>,
    stream_id: u64,
}

pub(crate) struct EventPage {
    pub(crate) events: Vec<Event>,
    pub(crate) next_cursor: String,
    pub(crate) has_more: bool,
    pub(crate) gap: bool,
    pub(crate) reset: bool,
}

impl EventLog {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                events: VecDeque::new(),
                next_id: 1,
            }),
            stream_id: RandomState::new().hash_one(0_u8),
        }
    }

    /// Appends one event, then drops what is older than the window or beyond
    /// the cap. `Unavailable` if the lock is poisoned or the clock is unusable.
    pub(crate) fn record(
        &self,
        route: Route,
        direction: Direction,
        action: Action,
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
    #[cfg(test)]
    pub(crate) fn recent(&self, limit: usize) -> Result<Vec<Event>, AppError> {
        let inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        Ok(recent_locked(&inner, now_ms()?, limit))
    }

    /// Continues from an opaque cursor. A cursor from another process or a
    /// previous run resets the stream; dropped events are reported as a gap.
    pub(crate) fn page(&self, cursor: Option<&str>) -> Result<EventPage, AppError> {
        let parsed = cursor.map(parse_cursor).transpose()?;
        let inner = self.inner.lock().map_err(|_| AppError::Unavailable)?;
        let now = now_ms()?;
        let cutoff = now.saturating_sub(WINDOW_MS);
        let reset = parsed.is_some_and(|(stream, _)| stream != self.stream_id);
        let after_id = parsed.filter(|_| !reset).map(|(_, id)| id);
        if after_id.is_some_and(|id| id >= inner.next_id) {
            return Err(AppError::InvalidBody);
        }
        let retained: Vec<&Event> = inner
            .events
            .iter()
            .filter(|event| event.timestamp_ms >= cutoff)
            .collect();
        let gap = after_id.map_or(retained.len() > MAX_EVENTS_PER_RESPONSE, |id| {
            let first_available = retained.first().map_or(inner.next_id, |event| event.id);
            id.saturating_add(1) < first_available
        });
        let mut events: Vec<Event> = if let Some(id) = after_id {
            retained
                .iter()
                .filter(|event| event.id > id)
                .take(MAX_EVENTS_PER_RESPONSE + 1)
                .map(|event| (*event).clone())
                .collect()
        } else {
            retained
                .iter()
                .rev()
                .take(MAX_EVENTS_PER_RESPONSE)
                .rev()
                .map(|event| (*event).clone())
                .collect()
        };
        let has_more = events.len() > MAX_EVENTS_PER_RESPONSE;
        if has_more {
            events.pop();
        }
        let last_id = events.last().map_or_else(
            || {
                after_id.map_or(inner.next_id.saturating_sub(1), |id| {
                    id.max(inner.next_id.saturating_sub(1))
                })
            },
            |event| event.id,
        );
        Ok(EventPage {
            events,
            next_cursor: format!("{:016x}:{last_id}", self.stream_id),
            has_more,
            gap,
            reset,
        })
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
        action: Action,
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

fn parse_cursor(cursor: &str) -> Result<(u64, u64), AppError> {
    let (stream, id) = cursor.split_once(':').ok_or(AppError::InvalidBody)?;
    if stream.len() != 16 || id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AppError::InvalidBody);
    }
    let stream = u64::from_str_radix(stream, 16).map_err(|_| AppError::InvalidBody)?;
    let id = id.parse().map_err(|_| AppError::InvalidBody)?;
    Ok((stream, id))
}

fn push_locked(
    inner: &mut Inner,
    now: u64,
    route: Route,
    direction: Direction,
    action: Action,
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

#[cfg(test)]
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

    use super::{
        Direction, Event, EventLog, Route, MAX_EVENTS, MAX_EVENTS_PER_RESPONSE, WINDOW_MS,
    };
    use crate::error::AppError;

    use super::Action;

    const T0: u64 = 10_000_000;

    fn push(log: &EventLog, now: u64, action: &'static str) {
        let action = match action {
            "allow" => Action::Allow,
            "mask" => Action::Mask,
            "block" => Action::Block,
            _ => panic!("unknown test action"),
        };
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
        assert_eq!(actions, vec![Action::Allow, Action::Mask, Action::Block]);
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
    fn cursor_reads_the_next_page_without_repeating_events() {
        let log = EventLog::new();
        for _ in 0..MAX_EVENTS_PER_RESPONSE + 2 {
            log.record(
                Route::DecisionsChat,
                Direction::Input,
                Action::Allow,
                vec![],
            )
            .expect("record");
        }
        let cursor = format!("{:016x}:0", log.stream_id);
        let first = log.page(Some(&cursor)).expect("first page");
        assert_eq!(first.events.len(), MAX_EVENTS_PER_RESPONSE);
        assert!(first.has_more);
        let second = log.page(Some(&first.next_cursor)).expect("second page");
        assert_eq!(second.events.len(), 2);
        assert!(!second.has_more);
        assert_eq!(second.events.first().map(|event| event.id), Some(1001));
    }

    #[test]
    fn cursor_from_another_run_resets_and_bad_cursor_is_rejected() {
        let log = EventLog::new();
        log.record(
            Route::DecisionsChat,
            Direction::Input,
            Action::Allow,
            vec![],
        )
        .expect("record");
        let other = EventLog::new();
        let cursor = other.page(None).expect("cursor").next_cursor;
        assert!(log.page(Some(&cursor)).expect("reset").reset);
        assert!(log.page(Some("invalid")).is_err());
    }

    #[test]
    fn recent_and_record_use_the_real_clock() {
        let log = EventLog::new();
        log.record(
            Route::ProxyChat,
            Direction::Output,
            Action::Block,
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
            action: Action::Mask,
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
            log.record(
                Route::DecisionsChat,
                Direction::Input,
                Action::Allow,
                vec![]
            ),
            Err(AppError::Unavailable)
        );
        assert_eq!(log.recent(10), Err(AppError::Unavailable));
    }

    fn cursor_at(log: &EventLog, id: u64) -> String {
        format!("{:016x}:{id}", log.stream_id)
    }

    #[test]
    fn cursor_reports_a_gap_when_unread_events_expired() {
        let log = EventLog::new();
        // T0 is in 1970: out of the window when page() reads the real clock.
        push(&log, T0, "allow");
        push(&log, T0 + 1, "allow");
        log.record(
            Route::DecisionsChat,
            Direction::Input,
            Action::Block,
            vec![],
        )
        .expect("record");

        let behind = log.page(Some(&cursor_at(&log, 0))).expect("page");
        assert!(behind.gap);
        assert_eq!(ids(&behind.events), vec![3]);

        let caught_up = log.page(Some(&cursor_at(&log, 2))).expect("page");
        assert!(!caught_up.gap);
    }

    #[test]
    fn cursor_reports_a_gap_when_the_cap_dropped_unread_events() {
        let log = EventLog::new();
        for _ in 0..MAX_EVENTS + 5 {
            log.record(
                Route::DecisionsChat,
                Direction::Input,
                Action::Allow,
                vec![],
            )
            .expect("record");
        }

        let behind = log.page(Some(&cursor_at(&log, 3))).expect("page");
        assert!(behind.gap);
        assert!(behind.has_more);
        assert_eq!(behind.events.first().map(|event| event.id), Some(6));

        // 5 was the last dropped id: a reader that saw it lost nothing.
        let caught_up = log.page(Some(&cursor_at(&log, 5))).expect("page");
        assert!(!caught_up.gap);
    }
}
