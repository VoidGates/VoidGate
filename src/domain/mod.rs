//! Pure decision rules. Nothing here knows about HTTP, JSON bodies or axum.
//! Content decisions live in `voidgate-core`; this module re-exports them so
//! the server keeps one path for every rule.

pub(crate) use voidgate_core::{action, detectors};
pub(crate) mod policy;
