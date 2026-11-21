//! JSON for `GET /v1/events`.

use serde::Serialize;

use crate::events::Event;

/// An object, not a bare list, so a field (a cursor, for instance) can be
/// added later without breaking whoever already reads `events`.
#[derive(Serialize)]
pub(crate) struct EventsResponse {
    pub(crate) events: Vec<Event>,
}
