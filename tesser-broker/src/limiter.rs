use std::num::NonZeroU32;
use std::sync::Arc;

use governor::{DefaultDirectRateLimiter, DefaultKeyedRateLimiter, Quota};
use thiserror::Error;

#[derive(Clone)]
pub struct RateLimiter {
    inner: RateLimiterKind,
}

#[derive(Clone)]
enum RateLimiterKind {
    Direct(Arc<DefaultDirectRateLimiter>),
    Keyed(Arc<DefaultKeyedRateLimiter<String>>),
}

#[derive(Debug, Error)]
pub enum RateLimiterError {
    #[error("this rate limiter requires a key identifier")]
    KeyRequired,
    #[error("this rate limiter does not accept keys")]
    UnexpectedKey,
    #[error("requested burst exceeds limiter capacity")]
    InsufficientCapacity,
}

impl RateLimiter {
    pub fn direct(quota: Quota) -> Self {
        Self {
            inner: RateLimiterKind::Direct(Arc::new(DefaultDirectRateLimiter::direct(quota))),
        }
    }

    pub fn keyed(quota: Quota) -> Self {
        Self {
            inner: RateLimiterKind::Keyed(Arc::new(DefaultKeyedRateLimiter::keyed(quota))),
        }
    }

    pub async fn until_ready(&self) -> Result<(), RateLimiterError> {
        match &self.inner {
            RateLimiterKind::Direct(inner) => {
                inner.until_ready().await;
                Ok(())
            }
            RateLimiterKind::Keyed(_) => Err(RateLimiterError::KeyRequired),
        }
    }

    pub async fn until_key_ready(&self, key: &str) -> Result<(), RateLimiterError> {
        match &self.inner {
            RateLimiterKind::Direct(_) => Err(RateLimiterError::UnexpectedKey),
            RateLimiterKind::Keyed(inner) => {
                inner.until_key_ready(&key.to_string()).await;
                Ok(())
            }
        }
    }

    pub async fn until_units_ready(&self, units: NonZeroU32) -> Result<(), RateLimiterError> {
        match &self.inner {
            RateLimiterKind::Direct(inner) => inner
                .until_n_ready(units)
                .await
                .map(|_| ())
                .map_err(|_| RateLimiterError::InsufficientCapacity),
            RateLimiterKind::Keyed(_) => Err(RateLimiterError::KeyRequired),
        }
    }

    pub async fn until_key_units_ready(
        &self,
        key: &str,
        units: NonZeroU32,
    ) -> Result<(), RateLimiterError> {
        match &self.inner {
            RateLimiterKind::Direct(_) => Err(RateLimiterError::UnexpectedKey),
            RateLimiterKind::Keyed(inner) => inner
                .until_key_n_ready(&key.to_string(), units)
                .await
                .map(|_| ())
                .map_err(|_| RateLimiterError::InsufficientCapacity),
        }
    }
}
