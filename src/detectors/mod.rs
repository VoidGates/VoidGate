//! Content detectors for chat decisions.
//!
//! Each detector inspects the message content and reports findings as byte
//! offsets. Detectors never copy or return the content itself, so a finding is
//! safe to serialize and log.

mod invisible_unicode;

use serde::Serialize;

pub(crate) use invisible_unicode::InvisibleUnicode;

/// A single suspicious span inside the inspected content.
///
/// `start` and `end` are byte offsets into the original UTF-8 content
/// (`end` is exclusive), so a caller can locate the span without the
/// content ever leaving the handler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Finding {
    pub(crate) code: &'static str,
    pub(crate) detector: &'static str,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Something that scans content and reports findings.
///
/// Implementations must be `Send + Sync` because they are shared between
/// request handlers running on different threads.
pub(crate) trait Detector: Send + Sync {
    /// Scans `content` and returns every finding, in offset order.
    fn scan(&self, content: &str) -> Vec<Finding>;
}

/// The detectors enabled for `POST /v1/decisions/chat`, in execution order.
///
/// New detectors are added here; the handler iterates this list and never
/// needs to know which detectors exist.
pub(crate) fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(InvisibleUnicode)]
}
