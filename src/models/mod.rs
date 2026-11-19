//! Request and response bodies (DTOs). No logic.

pub(crate) mod chat;
pub(crate) mod http;

use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) status: &'static str,
}
