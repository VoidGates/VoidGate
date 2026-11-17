//! The single error contract of the API.
//!
//! Every error leaves the service as `{"error":{"code","message"}}` with a
//! fixed code and a fixed message. Nothing here echoes the request or says
//! which validation rule failed: a caller learns the category of the mistake,
//! not how the defence works. A caller without a valid key only ever sees
//! `unauthorized`, whatever the route, method or body.

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppError {
    Unauthorized,
    NotFound,
    MethodNotAllowed,
    InvalidJson,
    InvalidBody,
    PayloadTooLarge,
    UnsupportedMediaType,
    RequestTimeout,
}

impl AppError {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 8] = [
        Self::Unauthorized,
        Self::NotFound,
        Self::MethodNotAllowed,
        Self::InvalidJson,
        Self::InvalidBody,
        Self::PayloadTooLarge,
        Self::UnsupportedMediaType,
        Self::RequestTimeout,
    ];

    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        self.parts().1
    }

    fn parts(self) -> (StatusCode, &'static str, &'static str) {
        match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Missing or invalid credentials.",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Unknown route."),
            Self::MethodNotAllowed => (
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "Method not supported for this route.",
            ),
            Self::InvalidJson => (
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "Request body is not valid JSON.",
            ),
            Self::InvalidBody => (
                StatusCode::BAD_REQUEST,
                "invalid_body",
                "Request body does not match the documented contract.",
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Request exceeds the documented size limit.",
            ),
            Self::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "Content-Type must be application/json.",
            ),
            Self::RequestTimeout => (
                StatusCode::REQUEST_TIMEOUT,
                "request_timeout",
                "Request took too long to arrive.",
            ),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = self.parts();
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

// The rejection's own text names fields and types; it is dropped on purpose.
impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        match rejection {
            JsonRejection::JsonSyntaxError(_) => Self::InvalidJson,
            JsonRejection::JsonDataError(_) => Self::InvalidBody,
            JsonRejection::MissingJsonContentType(_) => Self::UnsupportedMediaType,
            other if other.status() == StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            _ => Self::InvalidBody,
        }
    }
}

/// `Json` extractor whose rejection is an [`AppError`].
pub(crate) struct ApiJson<T>(pub(crate) T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(request, state).await?;
        Ok(Self(value))
    }
}

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
}

pub(crate) async fn method_not_allowed() -> AppError {
    AppError::MethodNotAllowed
}
