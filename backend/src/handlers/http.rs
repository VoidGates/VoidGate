//! `POST /v1/decisions/http`: decides whether a path goes to a decoy.

use axum::{extract::State, Json};

use crate::{
    error::{ApiJson, AppError},
    models::http::{HttpDecisionRequest, HttpDecisionResponse},
    services, AppState,
};

pub(crate) async fn decide_http(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<HttpDecisionRequest>,
) -> Result<Json<HttpDecisionResponse>, AppError> {
    Ok(Json(services::http::decide(&state.policy, &input)?))
}
