use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{error::AppError, AppState};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct ClientKey([u8; 32]);

impl ClientKey {
    #[cfg(test)]
    pub(crate) fn test_key(value: u8) -> Self {
        Self([value; 32])
    }
}

#[derive(Clone)]
pub(crate) struct ApiKeyVerifier([u8; 32]);

impl ApiKeyVerifier {
    pub(crate) fn new(api_key: &str) -> Result<Self, String> {
        if !(32..=128).contains(&api_key.len()) {
            return Err("VOIDGATE_API_KEY must contain 32 to 128 bytes".into());
        }
        // A key no header can carry would start a service that rejects everyone.
        if api_key != api_key.trim() || HeaderValue::from_str(api_key).is_err() {
            return Err(
                "VOIDGATE_API_KEY must not contain control characters or surrounding spaces".into(),
            );
        }
        Ok(Self(Sha256::digest(api_key.as_bytes()).into()))
    }

    fn verify(&self, supplied: Option<&HeaderValue>) -> bool {
        let Some(supplied) = supplied else {
            return false;
        };
        let supplied_digest: [u8; 32] = Sha256::digest(supplied.as_bytes()).into();
        bool::from(self.0[..].ct_eq(&supplied_digest[..]))
    }

    fn client_key(&self) -> ClientKey {
        ClientKey(self.0)
    }
}

pub(crate) async fn authorize(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if !state
        .key_verifier
        .verify(request.headers().get("x-voidgate-key"))
    {
        return Err(AppError::Unauthorized);
    }
    request
        .extensions_mut()
        .insert(state.key_verifier.client_key());
    Ok(next.run(request).await)
}
