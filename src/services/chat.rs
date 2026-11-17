use crate::{
    domain::detectors::{Detector, Finding},
    error::AppError,
    models::chat::{ChatDecisionRequest, ChatDecisionResponse},
};

/// Maximum size of `content`, in UTF-8 bytes.
pub(crate) const MAX_CONTENT_BYTES: usize = 16 * 1024;

/// Findings listed in one response; the decision still counts all of them.
pub(crate) const MAX_FINDINGS: usize = 100;

const ACTION_ALLOW: &str = "allow";
const ACTION_BLOCK: &str = "block";

pub(crate) fn decide(
    detectors: &[Box<dyn Detector>],
    input: &ChatDecisionRequest,
) -> Result<ChatDecisionResponse, AppError> {
    if input.content.len() > MAX_CONTENT_BYTES {
        return Err(AppError::PayloadTooLarge);
    }

    let mut findings: Vec<Finding> = detectors
        .iter()
        .flat_map(|detector| detector.scan(&input.content))
        .collect();

    let mut reason_codes: Vec<&'static str> = Vec::new();
    for finding in &findings {
        if !reason_codes.contains(&finding.code) {
            reason_codes.push(finding.code);
        }
    }
    let action = if findings.is_empty() {
        ACTION_ALLOW
    } else {
        ACTION_BLOCK
    };

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
