use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::Mutex, time::sleep};

use crate::application::error::AppError;

#[derive(Debug, Clone)]
pub struct LoginRateLimiter {
    state: Arc<Mutex<HashMap<LoginAttemptKey, AttemptState>>>,
    max_attempts: u32,
    window: Duration,
    block_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct IpRateLimiter {
    state: Arc<Mutex<HashMap<IpRateLimitKey, RequestState>>>,
    max_requests: u32,
    window: Duration,
    block_duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LoginAttemptKey {
    ip: IpAddr,
    username_normalized: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IpRateLimitKey {
    ip: IpAddr,
    scope: &'static str,
}

#[derive(Debug, Clone)]
struct AttemptState {
    attempts: u32,
    window_started_at: Instant,
    blocked_until: Option<Instant>,
}

#[derive(Debug, Clone)]
struct RequestState {
    requests: u32,
    window_started_at: Instant,
    blocked_until: Option<Instant>,
}

impl LoginRateLimiter {
    pub fn new(max_attempts: u32, window: Duration, block_duration: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window,
            block_duration,
        }
    }

    pub async fn check(&self, ip: IpAddr, username_normalized: &str) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        self.prune_expired(&mut state);

        let key = LoginAttemptKey {
            ip,
            username_normalized: username_normalized.to_owned(),
        };
        let Some(attempt) = state.get(&key) else {
            return Ok(());
        };

        if let Some(blocked_until) = attempt.blocked_until
            && blocked_until > Instant::now()
        {
            return Err(AppError::RateLimited);
        }

        Ok(())
    }

    pub async fn record_success(&self, ip: IpAddr, username_normalized: &str) {
        let mut state = self.state.lock().await;
        state.remove(&LoginAttemptKey {
            ip,
            username_normalized: username_normalized.to_owned(),
        });
    }

    pub async fn record_failure(&self, ip: IpAddr, username_normalized: &str) {
        let delay;

        {
            let mut state = self.state.lock().await;
            self.prune_expired(&mut state);

            let now = Instant::now();
            let key = LoginAttemptKey {
                ip,
                username_normalized: username_normalized.to_owned(),
            };
            let attempt = state.entry(key).or_insert_with(|| AttemptState {
                attempts: 0,
                window_started_at: now,
                blocked_until: None,
            });

            if now.duration_since(attempt.window_started_at) > self.window {
                attempt.attempts = 0;
                attempt.window_started_at = now;
                attempt.blocked_until = None;
            }

            attempt.attempts = attempt.attempts.saturating_add(1);
            delay = progressive_delay(attempt.attempts);

            if attempt.attempts >= self.max_attempts {
                attempt.blocked_until = Some(now + self.block_duration);
            }
        }

        if !delay.is_zero() {
            sleep(delay).await;
        }
    }

    fn prune_expired(&self, state: &mut HashMap<LoginAttemptKey, AttemptState>) {
        let now = Instant::now();
        state.retain(|_, attempt| {
            let window_active = now.duration_since(attempt.window_started_at) <= self.window;
            let block_active = attempt
                .blocked_until
                .is_some_and(|blocked_until| blocked_until > now);

            window_active || block_active
        });
    }
}

fn progressive_delay(attempts: u32) -> Duration {
    match attempts {
        0 | 1 => Duration::ZERO,
        2 => Duration::from_millis(250),
        3 => Duration::from_millis(500),
        _ => Duration::from_secs(1),
    }
}

impl IpRateLimiter {
    pub fn new(max_requests: u32, window: Duration, block_duration: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
            block_duration,
        }
    }

    pub async fn check_and_record(&self, ip: IpAddr, scope: &'static str) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        self.prune_expired(&mut state);

        let now = Instant::now();
        let key = IpRateLimitKey { ip, scope };
        let attempt = state.entry(key).or_insert_with(|| RequestState {
            requests: 0,
            window_started_at: now,
            blocked_until: None,
        });

        if let Some(blocked_until) = attempt.blocked_until
            && blocked_until > now
        {
            return Err(AppError::RateLimited);
        }

        if now.duration_since(attempt.window_started_at) > self.window {
            attempt.requests = 0;
            attempt.window_started_at = now;
            attempt.blocked_until = None;
        }

        attempt.requests = attempt.requests.saturating_add(1);
        if attempt.requests > self.max_requests {
            attempt.blocked_until = Some(now + self.block_duration);
            return Err(AppError::RateLimited);
        }

        Ok(())
    }

    fn prune_expired(&self, state: &mut HashMap<IpRateLimitKey, RequestState>) {
        let now = Instant::now();
        state.retain(|_, attempt| {
            let window_active = now.duration_since(attempt.window_started_at) <= self.window;
            let block_active = attempt
                .blocked_until
                .is_some_and(|blocked_until| blocked_until > now);

            window_active || block_active
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocks_after_configured_failures() {
        let limiter = LoginRateLimiter::new(2, Duration::from_secs(60), Duration::from_secs(60));
        let ip = "127.0.0.1".parse().unwrap();

        limiter.check(ip, "maicom").await.unwrap();
        limiter.record_failure(ip, "maicom").await;
        limiter.check(ip, "maicom").await.unwrap();
        limiter.record_failure(ip, "maicom").await;

        assert!(matches!(
            limiter.check(ip, "maicom").await,
            Err(AppError::RateLimited)
        ));
    }

    #[tokio::test]
    async fn success_clears_failures() {
        let limiter = LoginRateLimiter::new(2, Duration::from_secs(60), Duration::from_secs(60));
        let ip = "127.0.0.1".parse().unwrap();

        limiter.record_failure(ip, "maicom").await;
        limiter.record_success(ip, "maicom").await;

        assert!(limiter.check(ip, "maicom").await.is_ok());
    }

    #[tokio::test]
    async fn ip_rate_limiter_blocks_after_configured_requests() {
        let limiter = IpRateLimiter::new(2, Duration::from_secs(60), Duration::from_secs(60));
        let ip = "127.0.0.1".parse().unwrap();

        limiter.check_and_record(ip, "register").await.unwrap();
        limiter.check_and_record(ip, "register").await.unwrap();

        assert!(matches!(
            limiter.check_and_record(ip, "register").await,
            Err(AppError::RateLimited)
        ));
    }

    #[tokio::test]
    async fn ip_rate_limiter_keeps_scopes_separate() {
        let limiter = IpRateLimiter::new(1, Duration::from_secs(60), Duration::from_secs(60));
        let ip = "127.0.0.1".parse().unwrap();

        limiter.check_and_record(ip, "register").await.unwrap();

        assert!(limiter.check_and_record(ip, "mutation").await.is_ok());
    }
}
