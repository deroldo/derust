//! Password hashing with Argon2id (PHC string format).
//!
//! `hash` and `verify` are CPU- and memory-bound: in async code run them inside
//! `tokio::task::spawn_blocking`.

use crate::cryptox::error::PasswordError;
use crate::cryptox::util::random_bytes;
use argon2::password_hash::{
    Error as PhcError, PasswordHash, PasswordHasher as _, PasswordVerifier as _, SaltString,
};
use argon2::{Algorithm, Argon2, Params, Version};
use std::fmt;

const SALT_LEN: usize = 16;

/// Argon2id cost parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argon2Params {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for Argon2Params {
    /// OWASP minimum for Argon2id: 19 MiB, t = 2, p = 1.
    fn default() -> Self {
        Self {
            memory_kib: 19_456,
            iterations: 2,
            parallelism: 1,
        }
    }
}

/// Argon2id hasher with fixed parameters. `Debug` is redacted.
#[derive(Clone)]
pub struct PasswordHasher {
    params: Argon2Params,
    argon2: Argon2<'static>,
}

impl fmt::Debug for PasswordHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasswordHasher(***)")
    }
}

impl PasswordHasher {
    /// Fails with `InvalidParams` when the parameters are rejected by Argon2.
    pub fn new(params: Argon2Params) -> Result<Self, PasswordError> {
        let built = Params::new(
            params.memory_kib,
            params.iterations,
            params.parallelism,
            None,
        )
        .map_err(|_| PasswordError::InvalidParams)?;
        Ok(Self {
            params,
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, built),
        })
    }

    /// Returns a PHC string `$argon2id$v=19$m=..,t=..,p=..$salt$hash` with a fresh random salt.
    pub fn hash(&self, password: &str) -> Result<String, PasswordError> {
        let salt_bytes = random_bytes(SALT_LEN).map_err(|_| PasswordError::HashFailed)?;
        let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| PasswordError::HashFailed)?;
        self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| PasswordError::HashFailed)
    }

    /// Wrong password is `Ok(false)`. Malformed PHC is `Err(InvalidHash)`, a valid
    /// PHC of another algorithm is `Err(UnsupportedAlgorithm)`.
    ///
    /// The cost parameters come from the stored hash itself, which is treated as trusted.
    pub fn verify(&self, password: &str, phc: &str) -> Result<bool, PasswordError> {
        let parsed = PasswordHash::new(phc).map_err(|_| PasswordError::InvalidHash)?;
        if parsed.algorithm.as_str() != "argon2id" {
            return Err(PasswordError::UnsupportedAlgorithm);
        }
        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(PhcError::Password) => Ok(false),
            Err(_) => Err(PasswordError::InvalidHash),
        }
    }

    /// True when the stored hash was made with different parameters/algorithm/version
    /// than the configured ones, or cannot be read.
    pub fn needs_rehash(&self, phc: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(phc) else {
            return true;
        };
        if parsed.algorithm.as_str() != "argon2id" {
            return true;
        }
        if parsed.version != Some(Version::V0x13.into()) {
            return true;
        }
        match Params::try_from(&parsed) {
            Ok(p) => {
                p.m_cost() != self.params.memory_kib
                    || p.t_cost() != self.params.iterations
                    || p.p_cost() != self.params.parallelism
            }
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn small() -> Argon2Params {
        Argon2Params {
            memory_kib: 64,
            iterations: 1,
            parallelism: 1,
        }
    }

    fn hasher() -> PasswordHasher {
        PasswordHasher::new(small()).unwrap()
    }

    #[test]
    fn hash_and_verify() {
        let h = hasher();
        let phc = h.hash("correct horse").unwrap();
        assert!(h.verify("correct horse", &phc).unwrap());
        assert!(!h.verify("wrong", &phc).unwrap());
    }

    #[test]
    fn salts_differ_and_both_verify() {
        let h = hasher();
        let a = h.hash("pw").unwrap();
        let b = h.hash("pw").unwrap();
        assert_ne!(a, b);
        assert!(h.verify("pw", &a).unwrap());
        assert!(h.verify("pw", &b).unwrap());
    }

    #[test]
    fn default_params_phc_format() {
        let phc = PasswordHasher::new(Argon2Params::default())
            .unwrap()
            .hash("pw")
            .unwrap();
        assert!(phc.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{phc}");
    }

    #[test]
    fn corrupted_hash_is_invalid() {
        let h = hasher();
        assert_eq!(
            h.verify("pw", "not a hash"),
            Err(PasswordError::InvalidHash)
        );
        assert_eq!(h.verify("pw", ""), Err(PasswordError::InvalidHash));
        let mut phc = h.hash("pw").unwrap();
        phc.truncate(phc.len() - 10);
        phc.push_str("!!!!");
        assert!(h.verify("pw", &phc).is_err());
    }

    #[test]
    fn other_argon2_variants_are_unsupported() {
        let h = hasher();
        let salt = "c29tZXNhbHRzb21lc2FsdA";
        let hash = "1CPz4PuyeKkknCNTU5v52qlMt5LtwQsgM5BbeG7cH60";
        for alg in ["argon2i", "argon2d"] {
            let phc = format!("${alg}$v=19$m=64,t=3,p=2${salt}${hash}");
            assert_eq!(
                h.verify("password", &phc),
                Err(PasswordError::UnsupportedAlgorithm)
            );
        }
    }

    /// Fixture produced by an independent implementation (OpenSSL 4.1 `openssl kdf ARGON2ID`,
    /// pass="password", salt="somesaltsomesalt", t=3, m=64, p=2, 32 bytes).
    #[test]
    fn verifies_hash_from_independent_implementation() {
        let phc = "$argon2id$v=19$m=64,t=3,p=2$c29tZXNhbHRzb21lc2FsdA$1CPz4PuyeKkknCNTU5v52qlMt5LtwQsgM5BbeG7cH60";
        let h = hasher();
        assert!(h.verify("password", phc).unwrap());
        assert!(!h.verify("passwore", phc).unwrap());
    }

    #[test]
    fn needs_rehash_rules() {
        let h = hasher();
        let phc = h.hash("pw").unwrap();
        assert!(!h.needs_rehash(&phc));
        for other in [
            Argon2Params {
                memory_kib: 128,
                ..small()
            },
            Argon2Params {
                iterations: 2,
                ..small()
            },
            Argon2Params {
                parallelism: 2,
                ..small()
            },
        ] {
            assert!(PasswordHasher::new(other).unwrap().needs_rehash(&phc));
        }
        assert!(h.needs_rehash("garbage"));
        assert!(h.needs_rehash(""));
    }

    #[test]
    fn invalid_params_are_rejected() {
        let bad = [
            Argon2Params {
                parallelism: 0,
                ..small()
            },
            Argon2Params {
                memory_kib: 1,
                ..small()
            },
            Argon2Params {
                iterations: 0,
                ..small()
            },
        ];
        for p in bad {
            assert_eq!(
                PasswordHasher::new(p).unwrap_err(),
                PasswordError::InvalidParams
            );
        }
    }

    #[test]
    fn debug_and_errors_do_not_leak() {
        let h = hasher();
        let marker = "S3cr3t-marker-password";
        let phc = h.hash(marker).unwrap();
        let hash_part = phc.rsplit('$').next().unwrap().to_string();
        let dbg = format!("{h:?}");
        assert!(!dbg.contains(marker) && !dbg.contains(&hash_part));
        let err = h.verify(marker, &format!("{marker}{phc}x")).unwrap_err();
        let shown = format!("{err} {err:?}");
        assert!(!shown.contains(marker) && !shown.contains(&hash_part));
    }
}
