use crate::{
    domain::policy::{valid_path, Decision, ExactPathPolicy},
    error::AppError,
    models::http::{HttpDecisionRequest, HttpDecisionResponse},
};

pub(crate) fn decide(
    policy: &ExactPathPolicy,
    input: &HttpDecisionRequest,
) -> Result<HttpDecisionResponse, AppError> {
    if !valid_method(&input.method) || !valid_path(&input.path) {
        return Err(AppError::InvalidBody);
    }

    let (action, reason_code) = match policy.evaluate(&input.path) {
        Decision::Decoy => ("decoy", "registered_decoy_path"),
        Decision::Allow => ("allow", "no_matching_rule"),
    };

    Ok(HttpDecisionResponse {
        action,
        reason_code,
    })
}

fn valid_method(method: &str) -> bool {
    matches!(
        method,
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    )
}
