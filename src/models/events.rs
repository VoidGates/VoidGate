//! JSON for `GET /v1/events`.

use serde::Serialize;

use crate::events::EventPage;

/// An object, not a bare list, so a field (a cursor, for instance) can be
/// added later without breaking whoever already reads `events`.
#[derive(Serialize)]
pub(crate) struct EventsResponse {
    pub(crate) events: Vec<crate::events::Event>,
    pub(crate) next_cursor: String,
    pub(crate) has_more: bool,
    pub(crate) gap: bool,
    pub(crate) reset: bool,
}

impl From<EventPage> for EventsResponse {
    fn from(page: EventPage) -> Self {
        Self {
            events: page.events,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
            gap: page.gap,
            reset: page.reset,
        }
    }
}
