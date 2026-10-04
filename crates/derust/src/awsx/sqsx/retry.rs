use std::time::Duration;

/// SQS maximum visibility timeout (12 hours).
pub const SQS_MAX_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(43_200);

/// Exponential backoff with full jitter for failed SQS messages.
///
/// The delay before a failed message becomes visible again is
/// `random(0, min(max_delay, base_delay * factor^(n-1)))`, where `n` is the
/// message receive count (1 = first delivery). The ceiling never exceeds the
/// SQS limit of 12 hours.
///
/// Defaults: base 5s, max 15min, factor 2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy {
    base_delay: Duration,
    max_delay: Duration,
    factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(900),
            factor: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Creates a policy. A `factor` that is not finite or is below 1.0 behaves as 1.0
    /// (constant delay); a `max_delay` below `base_delay` caps the delay at `max_delay`.
    pub fn new(base_delay: Duration, max_delay: Duration, factor: f64) -> Self {
        Self {
            base_delay,
            max_delay,
            factor,
        }
    }

    /// Sets the base delay (delay ceiling of the first attempt).
    pub fn with_base_delay(mut self, base_delay: Duration) -> Self {
        self.base_delay = base_delay;
        self
    }

    /// Sets the maximum delay ceiling (never above 12 hours, the SQS limit).
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Sets the growth factor applied per attempt.
    pub fn with_factor(mut self, factor: f64) -> Self {
        self.factor = factor;
        self
    }

    /// Configured base delay.
    pub fn base_delay(&self) -> Duration {
        self.base_delay
    }

    /// Configured maximum delay.
    pub fn max_delay(&self) -> Duration {
        self.max_delay
    }

    /// Configured growth factor.
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// Delay ceiling (no jitter): `min(max, base * factor^(n-1))`, with `n` the
    /// receive count (0 is treated as 1), capped at 12 hours.
    pub fn max_delay_for(&self, receive_count: u32) -> Duration {
        if self.base_delay.is_zero() {
            return Duration::ZERO;
        }
        let factor = if self.factor.is_finite() && self.factor >= 1.0 {
            self.factor
        } else {
            1.0
        };
        let exp = receive_count.max(1).saturating_sub(1);
        let grown = self.base_delay.as_secs_f64() * factor.powi(exp.min(i32::MAX as u32) as i32);
        let cap = self.max_delay.min(SQS_MAX_VISIBILITY_TIMEOUT).as_secs_f64();
        let secs = if grown.is_finite() {
            grown.min(cap)
        } else {
            cap
        };
        Duration::from_secs_f64(secs.max(0.0))
    }

    /// Full jitter: a random delay in `[0, max_delay_for(receive_count)]`.
    pub fn delay_for(&self, receive_count: u32) -> Duration {
        self.delay_with_sample(receive_count, rand::random::<f64>())
    }

    /// Deterministic variant for tests; `sample` is clamped to `[0, 1]` and NaN becomes 0.
    pub(crate) fn delay_with_sample(&self, receive_count: u32, sample: f64) -> Duration {
        let sample = if sample.is_nan() {
            0.0
        } else {
            sample.clamp(0.0, 1.0)
        };
        Duration::from_secs_f64(self.max_delay_for(receive_count).as_secs_f64() * sample)
    }
}

/// Converts a delay to whole seconds for the SDK (truncates; saturates at 12 hours).
pub(crate) fn to_visibility_timeout_secs(delay: Duration) -> i32 {
    i32::try_from(delay.as_secs().min(SQS_MAX_VISIBILITY_TIMEOUT.as_secs()))
        .unwrap_or(SQS_MAX_VISIBILITY_TIMEOUT.as_secs() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn default_policy_is_5s_15min_factor_2() {
        let p = RetryPolicy::default();
        assert_eq!(p.base_delay(), s(5));
        assert_eq!(p.max_delay(), s(900));
        assert_eq!(p.factor(), 2.0);
    }

    #[test]
    fn max_delay_grows_exponentially_until_cap() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_delay_for(1), s(5));
        assert_eq!(p.max_delay_for(2), s(10));
        assert_eq!(p.max_delay_for(3), s(20));
        assert_eq!(p.max_delay_for(8), s(640));
        assert_eq!(p.max_delay_for(9), s(900));
        assert_eq!(p.max_delay_for(100), s(900));
    }

    #[test]
    fn receive_count_zero_is_treated_as_first_attempt() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_delay_for(0), p.max_delay_for(1));
    }

    #[test]
    fn no_overflow_on_huge_receive_count() {
        assert_eq!(RetryPolicy::default().max_delay_for(u32::MAX), s(900));
    }

    #[test]
    fn max_delay_is_clamped_to_12_hours() {
        let p = RetryPolicy::default().with_max_delay(s(100 * 3600));
        assert_eq!(p.max_delay_for(1000), s(43_200));
    }

    #[test]
    fn factor_below_one_or_non_finite_behaves_as_constant() {
        for f in [0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -2.0] {
            let p = RetryPolicy::new(s(5), s(900), f);
            for n in [1, 2, 10, u32::MAX] {
                assert_eq!(p.max_delay_for(n), s(5), "factor {f} n {n}");
            }
        }
    }

    #[test]
    fn zero_base_gives_zero_delay() {
        let p = RetryPolicy::default().with_base_delay(Duration::ZERO);
        assert_eq!(p.max_delay_for(5), Duration::ZERO);
        assert_eq!(p.max_delay_for(u32::MAX), Duration::ZERO);
    }

    #[test]
    fn max_below_base_caps_at_max() {
        let p = RetryPolicy::new(s(60), s(10), 2.0);
        assert_eq!(p.max_delay_for(1), s(10));
        assert_eq!(p.max_delay_for(5), s(10));
    }

    #[test]
    fn full_jitter_sample_bounds() {
        let p = RetryPolicy::default();
        assert_eq!(p.delay_with_sample(3, 0.0), Duration::ZERO);
        assert_eq!(p.delay_with_sample(3, 0.5), s(10));
        assert!(p.delay_with_sample(3, 0.999999) <= s(20));
        assert!(p.delay_with_sample(3, 1.5) <= s(20));
        assert_eq!(p.delay_with_sample(3, -1.0), Duration::ZERO);
        assert_eq!(p.delay_with_sample(3, f64::NAN), Duration::ZERO);
    }

    #[test]
    fn delay_for_stays_within_zero_and_ceiling() {
        let p = RetryPolicy::default();
        for _ in 0..1000 {
            assert!(p.delay_for(3) <= s(20));
        }
    }

    #[test]
    fn to_visibility_timeout_secs_truncates_and_saturates() {
        assert_eq!(to_visibility_timeout_secs(Duration::from_millis(1900)), 1);
        assert_eq!(to_visibility_timeout_secs(Duration::ZERO), 0);
        assert_eq!(to_visibility_timeout_secs(s(43_200)), 43_200);
        assert_eq!(to_visibility_timeout_secs(s(100 * 3600)), 43_200);
        assert_eq!(to_visibility_timeout_secs(Duration::MAX), 43_200);
    }
}
