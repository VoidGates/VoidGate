//! Chat decision. `block` wins over `mask`. Personal data alone is `mask`.

pub(crate) use voidgate_core::{action_of, scan as inspect, Verdict};

use crate::{
    domain::detectors::Detector,
    error::AppError,
    events::Direction,
    models::chat::{ChatDecisionRequest, ChatDecisionResponse, Role},
};

/// Maximum size of `content`, in UTF-8 bytes.
pub(crate) const MAX_CONTENT_BYTES: usize = 16 * 1024;

/// Findings listed in one response; the decision still counts all of them.
pub(crate) const MAX_FINDINGS: usize = 100;

/// What the assistant says is `output`; what the user or the system sends is `input`.
pub(crate) fn direction_of(role: &Role) -> Direction {
    match role {
        Role::Assistant => Direction::Output,
        Role::User | Role::System => Direction::Input,
    }
}

pub(crate) fn decide(
    detectors: &[Box<dyn Detector>],
    input: &ChatDecisionRequest,
) -> Result<ChatDecisionResponse, AppError> {
    if input.content.len() > MAX_CONTENT_BYTES {
        return Err(AppError::PayloadTooLarge);
    }

    let mut findings = inspect(detectors, &input.content);
    // Folded before truncating: the decision still counts every finding.
    let mut verdict = Verdict::allow();
    verdict.absorb(&findings);

    // Bounds the response: without it a 16 KiB message could ask for one of ~400 KiB.
    let truncated = findings.len() > MAX_FINDINGS;
    findings.truncate(MAX_FINDINGS);

    Ok(ChatDecisionResponse {
        action: verdict.action,
        reason_codes: verdict.reason_codes,
        findings,
        truncated,
    })
}
