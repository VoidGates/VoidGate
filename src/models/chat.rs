use serde::{Deserialize, Serialize};

use crate::domain::detectors::Finding;

// No `Debug` and no `Serialize` on purpose: either would make it easy to
// print or echo the content.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChatDecisionRequest {
    // Validated on parse so the contract is enforced now; no detector reads
    // it yet, which is why the compiler would otherwise flag it as unused.
    #[allow(dead_code)]
    pub(crate) role: Role,
    pub(crate) content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    User,
    System,
    Assistant,
}

#[derive(Serialize)]
pub(crate) struct ChatDecisionResponse {
    pub(crate) action: &'static str,
    pub(crate) reason_codes: Vec<&'static str>,
    pub(crate) findings: Vec<Finding>,
}
