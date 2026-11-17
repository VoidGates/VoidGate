use crate::{
    domain::detectors::{Detector, Finding},
    error::AppError,
    models::chat::{ChatDecisionRequest, ChatDecisionResponse},
};

/// Maximum size of `content`, in UTF-8 bytes.
pub(crate) const MAX_CONTENT_BYTES: usize = 16 * 1024;

const ACTION_ALLOW: &str = "allow";
const ACTION_BLOCK: &str = "block";

pub(crate) fn decide(
    detectors: &[Box<dyn Detector>],
    input: &ChatDecisionRequest,
) -> Result<ChatDecisionResponse, AppError> {
    if input.content.len() > MAX_CONTENT_BYTES {
        return Err(AppError::PayloadTooLarge);
    }

    let findings: Vec<Finding> = detectors
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

    Ok(ChatDecisionResponse {
        action,
        reason_codes,
        findings,
    })
}
