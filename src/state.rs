//! Read-only configuration shared by every request. Built once at startup.

use std::sync::Arc;

use crate::{
    auth::ApiKeyVerifier,
    domain::{
        detectors::{default_detectors, Detector},
        policy::ExactPathPolicy,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub(crate) key_verifier: ApiKeyVerifier,
    pub(crate) policy: Arc<ExactPathPolicy>,
    pub(crate) detectors: Arc<Vec<Box<dyn Detector>>>,
}

impl AppState {
    pub fn new(api_key: String, decoy_paths: Vec<String>) -> Result<Self, String> {
        Ok(Self {
            key_verifier: ApiKeyVerifier::new(&api_key)?,
            policy: Arc::new(ExactPathPolicy::new(decoy_paths)?),
            detectors: Arc::new(default_detectors()),
        })
    }
}
