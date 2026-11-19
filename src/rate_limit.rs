//! Per-process token bucket for authenticated decision requests.
//!
//! Each configured credential gets an independent bucket. The bounded map is
//! ready for a multi-key verifier; this service currently accepts one key.
//! State is local to a process and must move to shared storage for global limits.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{auth::ClientKey, error::AppError, AppState};

const WINDOW: Duration = Duration::from_secs(60);
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 60;
const MAX_REQUESTS_PER_MINUTE: u32 = 10_000;
const MAX_CLIENT_BUCKETS: usize = 1_024;

#[derive(Clone, Copy)]
pub struct RateLimitConfig {
    requests_per_minute: u32,
}

impl RateLimitConfig {
    pub fn per_minute(requests: u32) -> Result<Self, String> {
        if !(1..=MAX_REQUESTS_PER_MINUTE).contains(&requests) {
            return Err(format!(
                "VOIDGATE_RATE_LIMIT_PER_MINUTE must be between 1 and {MAX_REQUESTS_PER_MINUTE}"
            ));
        }
        Ok(Self {
            requests_per_minute: requests,
        })
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_REQUESTS_PER_MINUTE,
        }
    }
}

struct Bucket {
    tokens: f64,
    updated_at: Instant,
}

pub(crate) struct RateLimiter {
    capacity: f64,
    buckets: Mutex<HashMap<ClientKey, Bucket>>,
}

enum Decision {
    Allow,
    Reject { retry_after_seconds: u64 },
}

impl RateLimiter {
    pub(crate) fn new(config: RateLimitConfig) -> Self {
        Self {
            capacity: f64::from(config.requests_per_minute),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn check(&self, client: ClientKey) -> Result<Decision, StatusCode> {
        // Read time while holding the lock so concurrent callers cannot move
        // the bucket timestamp backwards after acquiring the lock out of order.
        let mut buckets = self
            .buckets
            .lock()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        self.check_locked(&mut buckets, client, Instant::now())
    }

    #[cfg(test)]
    fn check_at(&self, client: ClientKey, now: Instant) -> Result<Decision, StatusCode> {
        let mut buckets = self
            .buckets
            .lock()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        self.check_locked(&mut buckets, client, now)
    }

    fn check_locked(
        &self,
        buckets: &mut HashMap<ClientKey, Bucket>,
        client: ClientKey,
        now: Instant,
    ) -> Result<Decision, StatusCode> {
        if !buckets.contains_key(&client) && buckets.len() >= MAX_CLIENT_BUCKETS {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let bucket = buckets.entry(client).or_insert_with(|| Bucket {
            tokens: self.capacity,
            updated_at: now,
        });
        let now = now.max(bucket.updated_at);
        let elapsed = now
            .saturating_duration_since(bucket.updated_at)
            .as_secs_f64();
        let refill_per_second = self.capacity / WINDOW.as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * refill_per_second).min(self.capacity);
        bucket.updated_at = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(Decision::Allow)
        } else {
            let wait = (1.0 - bucket.tokens) / refill_per_second;
            Ok(Decision::Reject {
                retry_after_seconds: (wait.ceil() as u64).max(1),
            })
        }
    }

    pub(crate) fn is_available(&self) -> bool {
        self.buckets.lock().is_ok()
    }
}

pub(crate) async fn enforce(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(client) = request.extensions().get::<ClientKey>().copied() else {
        return Err(AppError::Unavailable);
    };
    state
        .metrics
        .authenticated_decisions
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match state
        .rate_limiter
        .check(client)
        .map_err(|_| AppError::Unavailable)?
    {
        Decision::Allow => Ok(next.run(request).await),
        Decision::Reject {
            retry_after_seconds,
        } => {
            state
                .metrics
                .rate_limit_rejections
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let value = HeaderValue::from_str(&retry_after_seconds.to_string())
                .map_err(|_| AppError::Unavailable)?;
            let mut response = AppError::RateLimited.into_response();
            response.headers_mut().insert(RETRY_AFTER, value);
            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        thread,
        time::{Duration, Instant},
    };

    use axum::http::StatusCode;

    use super::{Decision, RateLimitConfig, RateLimiter};

    fn key(value: u8) -> crate::auth::ClientKey {
        crate::auth::ClientKey::test_key(value)
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert!(RateLimitConfig::per_minute(0).is_err());
        assert!(RateLimitConfig::per_minute(10_001).is_err());
        assert!(RateLimitConfig::per_minute(1).is_ok());
    }

    #[test]
    fn allows_burst_then_refills_without_resetting_the_window() {
        let limiter = RateLimiter::new(RateLimitConfig::per_minute(2).expect("valid limit"));
        let start = Instant::now();
        assert!(matches!(
            limiter.check_at(key(1), start),
            Ok(Decision::Allow)
        ));
        assert!(matches!(
            limiter.check_at(key(1), start),
            Ok(Decision::Allow)
        ));
        assert!(matches!(
            limiter.check_at(key(1), start),
            Ok(Decision::Reject {
                retry_after_seconds: 30
            })
        ));
        assert!(matches!(
            limiter.check_at(key(1), start + Duration::from_secs(29)),
            Ok(Decision::Reject {
                retry_after_seconds: 1
            })
        ));
        assert!(matches!(
            limiter.check_at(key(1), start + Duration::from_secs(31)),
            Ok(Decision::Allow)
        ));
        assert!(matches!(
            limiter.check_at(key(1), start + Duration::from_secs(31)),
            Ok(Decision::Reject { .. })
        ));
    }

    #[test]
    fn each_client_has_an_independent_bucket() {
        let limiter = RateLimiter::new(RateLimitConfig::per_minute(1).expect("valid limit"));
        let instant = Instant::now();
        assert!(matches!(
            limiter.check_at(key(1), instant),
            Ok(Decision::Allow)
        ));
        assert!(matches!(
            limiter.check_at(key(1), instant),
            Ok(Decision::Reject { .. })
        ));
        assert!(matches!(
            limiter.check_at(key(2), instant),
            Ok(Decision::Allow)
        ));
    }

    #[test]
    fn concurrent_requests_cannot_exceed_capacity() {
        let limiter = Arc::new(RateLimiter::new(
            RateLimitConfig::per_minute(5).expect("valid limit"),
        ));
        let instant = Instant::now();
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
                thread::spawn(move || {
                    matches!(limiter.check_at(key(1), instant), Ok(Decision::Allow))
                })
            })
            .collect();
        let accepted = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker completed"))
            .filter(|accepted| *accepted)
            .count();
        assert_eq!(accepted, 5);
    }

    #[test]
    fn poisoned_state_returns_service_unavailable() {
        let limiter = Arc::new(RateLimiter::new(RateLimitConfig::default()));
        let poisoned = Arc::clone(&limiter);
        let _ = thread::spawn(move || {
            let _guard = poisoned.buckets.lock().expect("lock");
            panic!("poison buckets");
        })
        .join();
        assert!(matches!(
            limiter.check(key(1)),
            Err(StatusCode::SERVICE_UNAVAILABLE)
        ));
    }
}
