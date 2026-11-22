//! Content detectors for chat decisions.
//!
//! Each detector inspects the message content and reports findings as byte
//! offsets. Detectors never copy or return the content itself, so a finding is
//! safe to serialize and log.

pub mod invisible_unicode;
pub mod manipulation;
pub mod personal_data;

pub use invisible_unicode::InvisibleUnicode;
pub use manipulation::PromptInjection;
pub use personal_data::PersonalData;

/// A single suspicious span inside the inspected content.
///
/// `start` and `end` are byte offsets into the original UTF-8 content
/// (`end` is exclusive), so a caller can locate the span without the
/// finding ever carrying the content itself.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Finding {
    pub code: &'static str,
    pub detector: &'static str,
    pub start: usize,
    pub end: usize,
}

/// Something that scans content and reports findings.
///
/// Implementations must be `Send + Sync` because they are shared between
/// threads (request handlers, agent workers).
pub trait Detector: Send + Sync {
    /// Scans `content` and returns every finding, in offset order.
    fn scan(&self, content: &str) -> Vec<Finding>;
}

/// Every built-in detector, in execution order. New detectors are added here.
pub fn default_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(InvisibleUnicode),
        Box::new(PromptInjection),
        Box::new(PersonalData),
    ]
}
