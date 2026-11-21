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

/// The outcome of a whole exchange: the most severe action seen
/// (`block` > `mask` > `allow`) and the reason codes of every piece, each once.
/// The proxy builds one per request and one per model reply.
pub(crate) struct Verdict {
    pub(crate) action: &'static str,
    pub(crate) reason_codes: Vec<&'static str>,
}

impl Verdict {
    pub(crate) fn allow() -> Self {
        Self {
            action: ACTION_ALLOW,
            reason_codes: Vec::new(),
        }
    }

    /// Folds in the findings of one more piece of text.
    pub(crate) fn absorb(&mut self, findings: &[Finding]) {
        let action = action_of(findings);
        if severity(action) > severity(self.action) {
            self.action = action;
        }
        for finding in findings {
            if !self.reason_codes.contains(&finding.code) {
                self.reason_codes.push(finding.code);
            }
        }
    }
}

fn severity(action: &str) -> u8 {
    match action {
        ACTION_BLOCK => 2,
        ACTION_MASK => 1,
        _ => 0,
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
