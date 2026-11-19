use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpDecisionRequest {
    pub(crate) method: String,
    pub(crate) path: String,
}

#[derive(Serialize)]
pub(crate) struct HttpDecisionResponse {
    pub(crate) action: &'static str,
    pub(crate) reason_code: &'static str,
}
