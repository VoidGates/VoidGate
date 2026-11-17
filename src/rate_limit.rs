//! Per-process token bucket for authenticated decision requests.
//!
//! The bucket is deliberately small and independent of the HTTP layer so a
//! shared store can replace it when requests are spread across replicas.

use std::{
    str::FromStr,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::AppState;

const WINDOW: Duration = Duration::from_secs(60);
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 60;
const MAX_REQUESTS_PER_MINUTE: u32 = 10_000;

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
    bucket: Mutex<Bucket>,
}

enum Decision {
    Allow,
    Reject { retry_after_seconds: u64 },
}

impl RateLimiter {
    pub(crate) fn new(config: RateLimitConfig) -> Self {
        Self {
            capacity: f64::from(config.requests_per_minute),
            bucket: Mutex::new(Bucket {
                tokens: f64::from(config.requests_per_minute),
                updated_at: Instant::now(),
            }),
        }
    }

    fn check(&self) -> Result<Decision, StatusCode> {
        self.check_at(Instant::now())
    }

    fn check_at(&self, now: Instant) -> Result<Decision, StatusCode> {
        // A poisoned lock means the limiter's state is unreliable. Fail closed.
        let mut bucket = self
            .bucket
            .lock()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let elapsed = now.saturating_duration_since(bucket.updated_at).as_secs_f64();
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
}

pub(crate) async fn enforce(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match state.rate_limiter.check()? {
        Decision::Allow => Ok(next.run(request).await),
        Decision::Reject {
            retry_after_seconds,
        } => {
            let value = HeaderValue::from_str(&retry_after_seconds.to_string())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
            response.headers_mut().insert(RETRY_AFTER, value);
            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use axum::http::StatusCode;

    use super::{Decision, RateLimitConfig, RateLimiter};

    #[test]
    fn rejects_invalid_configuration() {
        assert!(RateLimitConfig::per_minute(0).is_err());
        assert!(RateLimitConfig::per_minute(10_001).is_err());
        assert!(RateLimitConfig::per_minute(1).is_ok());
    }

    #[test]
    fn allows_burst_then_refills_without_resetting_the_window() {
        let limiter = RateLimiter::new(RateLimitConfig::per_minute(2).expect("valid limit"));
        let start = limiter.bucket.lock().expect("lock").updated_at;
        assert!(matches!(limiter.check_at(start), Ok(Decision::Allow)));
        assert!(matches!(limiter.check_at(start), Ok(Decision::Allow)));
        assert!(matches!(
            limiter.check_at(start),
            Ok(Decision::Reject {
                retry_after_seconds: 30
            })
        ));
        assert!(matches!(
            limiter.check_at(start + Duration::from_secs(29)),
            Ok(Decision::Reject {
                retry_after_seconds: 1
            })
        ));
        assert!(matches!(
            limiter.check_at(start + Duration::from_secs(31)),
            Ok(Decision::Allow)
        ));
        assert!(matches!(
            limiter.check_at(start + Duration::from_secs(31)),
            Ok(Decision::Reject { .. })
        ));
    }

    #[test]
    fn concurrent_requests_cannot_exceed_capacity() {
        let limiter = Arc::new(RateLimiter::new(
            RateLimitConfig::per_minute(5).expect("valid limit"),
        ));
        let instant = std::time::Instant::now();
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
                thread::spawn(move || matches!(limiter.check_at(instant), Ok(Decision::Allow)))
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
            let _guard = poisoned.bucket.lock().expect("lock");
            panic!("poison bucket");
        })
        .join();
        assert!(matches!(
            limiter.check(),
            Err(StatusCode::SERVICE_UNAVAILABLE)
        ));
    }
}
