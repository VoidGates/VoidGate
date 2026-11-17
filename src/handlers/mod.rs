//! HTTP layer: extract the request, call a service, return JSON.

pub(crate) mod chat;
#[cfg(debug_assertions)]
pub(crate) mod docs;
pub(crate) mod health;
pub(crate) mod http;
