use std::time::Duration;

pub(super) const DEFAULT_KEY_ATTRIBUTE: &str = "x-idempotent-key";
pub(super) const DEFAULT_COMPLETED_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const DEFAULT_IN_PROGRESS_TTL: Duration = Duration::from_secs(5 * 60);
const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const DEFAULT_CLEANUP_BATCH_SIZE: u32 = 1000;

/// Invalid idempotency configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyConfigError {
    /// A TTL (or interval) is zero.
    InvalidTtl,
    /// The key attribute name is empty or blank.
    EmptyKeyAttribute,
    /// The cleanup batch size is zero.
    InvalidBatchSize,
    /// The table name is not a valid (optionally schema-qualified) identifier.
    InvalidTableName,
    /// `with_idempotency` was called on a consumer that is already transactional.
    AlreadyTransactional,
}

impl std::fmt::Display for IdempotencyConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::InvalidTtl => "idempotency TTLs and intervals must be greater than zero",
            Self::EmptyKeyAttribute => "idempotency key attribute must not be empty",
            Self::InvalidBatchSize => "idempotency cleanup batch size must be greater than zero",
            Self::InvalidTableName => "invalid idempotency table name",
            Self::AlreadyTransactional => "consumer is already transactional",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for IdempotencyConfigError {}

/// Background cleanup of expired idempotency records. Disabled unless set on
/// [`IdempotencyConfig::with_cleanup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupConfig {
    /// Time between cleanup runs. Default: 1 hour.
    pub interval: Duration,
    /// Maximum number of records removed per store call. Default: 1000.
    pub batch_size: u32,
}

impl CleanupConfig {
    /// Checks that the interval and the batch size are greater than zero.
    pub(super) fn validate(&self) -> Result<(), IdempotencyConfigError> {
        if self.interval.is_zero() {
            return Err(IdempotencyConfigError::InvalidTtl);
        }
        if self.batch_size == 0 {
            return Err(IdempotencyConfigError::InvalidBatchSize);
        }
        Ok(())
    }
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            interval: DEFAULT_CLEANUP_INTERVAL,
            batch_size: DEFAULT_CLEANUP_BATCH_SIZE,
        }
    }
}

/// Idempotency settings of an SQS consumer. The `with_*` setters never fail; call
/// [`IdempotencyConfig::validate`] (done by the consumer) to check the values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyConfig {
    /// Message attribute holding the idempotency key. Default: `x-idempotent-key`.
    /// When absent or blank, the SQS `message_id` is used instead.
    pub key_attribute: String,
    /// How long a successfully processed key is remembered. Default: 7 days.
    /// It should cover the redelivery/republishing window (SQS retains messages up to 14 days).
    pub completed_ttl: Duration,
    /// How long a key stays reserved while a handler runs. Default: 5 minutes.
    /// It must be greater than the maximum handler time and aligned with the queue's
    /// visibility timeout; if it expires while the handler still runs, a duplicate may execute.
    pub in_progress_ttl: Duration,
    /// Background cleanup of expired records. Default: `None` (disabled).
    pub cleanup: Option<CleanupConfig>,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            key_attribute: DEFAULT_KEY_ATTRIBUTE.to_string(),
            completed_ttl: DEFAULT_COMPLETED_TTL,
            in_progress_ttl: DEFAULT_IN_PROGRESS_TTL,
            cleanup: None,
        }
    }
}

impl IdempotencyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key_attribute(mut self, key_attribute: impl Into<String>) -> Self {
        self.key_attribute = key_attribute.into();
        self
    }

    pub fn with_completed_ttl(mut self, ttl: Duration) -> Self {
        self.completed_ttl = ttl;
        self
    }

    pub fn with_in_progress_ttl(mut self, ttl: Duration) -> Self {
        self.in_progress_ttl = ttl;
        self
    }

    pub fn with_cleanup(mut self, cleanup: CleanupConfig) -> Self {
        self.cleanup = Some(cleanup);
        self
    }

    /// Checks that TTLs are greater than zero, the key attribute is not blank and, when
    /// cleanup is set, its interval and batch size are greater than zero.
    pub fn validate(&self) -> Result<(), IdempotencyConfigError> {
        if self.completed_ttl.is_zero() || self.in_progress_ttl.is_zero() {
            return Err(IdempotencyConfigError::InvalidTtl);
        }
        if self.key_attribute.trim().is_empty() {
            return Err(IdempotencyConfigError::EmptyKeyAttribute);
        }
        if let Some(cleanup) = &self.cleanup {
            cleanup.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = IdempotencyConfig::default();
        assert_eq!(c.key_attribute, "x-idempotent-key");
        assert_eq!(c.completed_ttl, Duration::from_secs(7 * 86_400));
        assert_eq!(c.in_progress_ttl, Duration::from_secs(300));
        assert!(c.cleanup.is_none());
        assert_eq!(c.validate(), Ok(()));
        let cl = CleanupConfig::default();
        assert_eq!(cl.interval, Duration::from_secs(3600));
        assert_eq!(cl.batch_size, 1000);
    }

    #[test]
    fn setters() {
        let c = IdempotencyConfig::new()
            .with_key_attribute("k")
            .with_completed_ttl(Duration::from_secs(1))
            .with_in_progress_ttl(Duration::from_secs(2))
            .with_cleanup(CleanupConfig {
                interval: Duration::from_secs(3),
                batch_size: 4,
            });
        assert_eq!(c.key_attribute, "k");
        assert_eq!(c.completed_ttl, Duration::from_secs(1));
        assert_eq!(c.in_progress_ttl, Duration::from_secs(2));
        assert_eq!(c.cleanup.as_ref().unwrap().batch_size, 4);
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn rejects_invalid() {
        let zero = Duration::ZERO;
        assert_eq!(
            IdempotencyConfig::new().with_completed_ttl(zero).validate(),
            Err(IdempotencyConfigError::InvalidTtl)
        );
        assert_eq!(
            IdempotencyConfig::new()
                .with_in_progress_ttl(zero)
                .validate(),
            Err(IdempotencyConfigError::InvalidTtl)
        );
        assert_eq!(
            IdempotencyConfig::new().with_key_attribute("").validate(),
            Err(IdempotencyConfigError::EmptyKeyAttribute)
        );
        assert_eq!(
            IdempotencyConfig::new().with_key_attribute("  ").validate(),
            Err(IdempotencyConfigError::EmptyKeyAttribute)
        );
        let bad_batch = CleanupConfig {
            interval: Duration::from_secs(1),
            batch_size: 0,
        };
        assert_eq!(
            IdempotencyConfig::new().with_cleanup(bad_batch).validate(),
            Err(IdempotencyConfigError::InvalidBatchSize)
        );
        let bad_interval = CleanupConfig {
            interval: zero,
            batch_size: 1,
        };
        assert_eq!(
            IdempotencyConfig::new()
                .with_cleanup(bad_interval)
                .validate(),
            Err(IdempotencyConfigError::InvalidTtl)
        );
    }

    #[test]
    fn error_display() {
        assert!(!IdempotencyConfigError::InvalidTableName
            .to_string()
            .is_empty());
        let _: &dyn std::error::Error = &IdempotencyConfigError::AlreadyTransactional;
    }
}
