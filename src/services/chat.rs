//! Chat decision. `block` wins over `mask`. Personal data alone is `mask`.

use crate::{
    domain::detectors::{personal_data, Detector, Finding},
    error::AppError,
    events::Direction,
    models::chat::{ChatDecisionRequest, ChatDecisionResponse, Role},
};

/// Maximum size of `content`, in UTF-8 bytes.
pub(crate) const MAX_CONTENT_BYTES: usize = 16 * 1024;

/// Findings listed in one response; the decision still counts all of them.
pub(crate) const MAX_FINDINGS: usize = 100;

const ACTION_ALLOW: &str = "allow";
const ACTION_BLOCK: &str = "block";
const ACTION_MASK: &str = "mask";

pub(crate) fn inspect(detectors: &[Box<dyn Detector>], content: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = detectors
        .iter()
        .flat_map(|detector| detector.scan(content))
        .collect();
    findings.sort_by_key(|finding| (finding.start, finding.end));
    findings
}

/// `block` wins over `mask`. Any code other than personal data blocks.
pub(crate) fn action_of(findings: &[Finding]) -> &'static str {
    if findings
        .iter()
        .any(|finding| finding.code != personal_data::REASON_CODE)
    {
        ACTION_BLOCK
    } else if findings.is_empty() {
        ACTION_ALLOW
    } else {
        ACTION_MASK
    }
}

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
    let action = action_of(&findings);
    let mut reason_codes: Vec<&'static str> = Vec::new();
    for finding in &findings {
        if !reason_codes.contains(&finding.code) {
            reason_codes.push(finding.code);
        }
    }

    // Bounds the response: without it a 16 KiB message could ask for one of ~400 KiB.
    let truncated = findings.len() > MAX_FINDINGS;
    findings.truncate(MAX_FINDINGS);

    Ok(ChatDecisionResponse {
        action,
        reason_codes,
        findings,
        truncated,
    })
}
