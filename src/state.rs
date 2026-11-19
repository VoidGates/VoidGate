//! Read-only configuration shared by every request. Built once at startup.

use std::{sync::Arc, time::Duration};

use tokio::sync::Semaphore;

use crate::{
    auth::ApiKeyVerifier,
    domain::{
        detectors::{default_detectors, Detector},
        policy::ExactPathPolicy,
    },
    metrics::Metrics,
    rate_limit::{RateLimitConfig, RateLimiter},
};

/// Decisions in flight at once; beyond this, callers get `503`.
const MAX_CONCURRENT_DECISIONS: usize = 256;

#[derive(Clone)]
pub struct AppState {
    pub(crate) key_verifier: ApiKeyVerifier,
    pub(crate) policy: Arc<ExactPathPolicy>,
    pub(crate) detectors: Arc<Vec<Box<dyn Detector>>>,
    pub(crate) rate_limiter: Arc<RateLimiter>,
    pub(crate) decision_slots: Arc<Semaphore>,
    pub(crate) metrics: Arc<Metrics>,
    pub(crate) upstream_url: Option<String>,
    pub(crate) http: reqwest::Client,
}

impl AppState {
    pub fn new(api_key: String, decoy_paths: Vec<String>) -> Result<Self, String> {
        Self::with_rate_limit(api_key, decoy_paths, RateLimitConfig::default())
    }

    pub fn with_rate_limit(
        api_key: String,
        decoy_paths: Vec<String>,
        rate_limit: RateLimitConfig,
    ) -> Result<Self, String> {
        Ok(Self {
            key_verifier: ApiKeyVerifier::new(&api_key)?,
            policy: Arc::new(ExactPathPolicy::new(decoy_paths)?),
            detectors: Arc::new(default_detectors()),
            rate_limiter: Arc::new(RateLimiter::new(rate_limit)),
            decision_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_DECISIONS)),
            metrics: Arc::new(Metrics::default()),
            upstream_url: None,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|error| format!("cannot prepare the upstream client ({error})"))?,
        })
    }

    /// `None` leaves `POST /v1/proxy/chat` failing closed.
    pub fn with_upstream(mut self, upstream_url: Option<String>) -> Result<Self, String> {
        if let Some(url) = &upstream_url {
            let ok = (url.starts_with("https://") || url.starts_with("http://"))
                && !url
                    .chars()
                    .any(|char| char.is_whitespace() || char.is_control())
                && !url.contains('@');
            if !ok {
                return Err("VOIDGATE_UPSTREAM_URL must be an http(s) URL".into());
            }
        }
        self.upstream_url = upstream_url;
        Ok(self)
    }
}
