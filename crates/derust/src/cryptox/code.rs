//! Short numeric codes (OTP-like) and their HMAC-SHA256 + pepper storage hash.
//!
//! Argon2 is the wrong tool for short codes: a 6-digit space is tiny, so a stolen
//! hash would be brute-forced regardless of the KDF cost. A keyed HMAC with a pepper
//! kept outside the database makes the stored hash useless without the pepper.
//! The online defense (attempt limits and TTL) belongs to the calling service.

use crate::cryptox::util::{hex_decode, hex_encode};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use std::fmt;
use zeroize::Zeroizing;

const HMAC_SHA256_LEN: usize = 32;

/// Generates a uniformly random numeric code with `digits` digits (leading zeros kept).
/// Uses rejection sampling, so there is no modulo bias for any `digits`.
/// `digits == 0` returns an empty string.
pub fn generate_numeric_code(digits: usize) -> String {
    digits_from_source(digits, || {
        let mut byte = [0u8; 1];
        loop {
            // OpenSSL RNG failure is not recoverable; retry instead of panicking.
            if openssl::rand::rand_bytes(&mut byte).is_ok() {
                return byte[0];
            }
        }
    })
}

/// Byte-by-byte rejection sampling: bytes 250..=255 are discarded so that 0..=249
/// maps exactly 25 times onto each digit.
pub(crate) fn digits_from_source(n: usize, mut next_byte: impl FnMut() -> u8) -> String {
    let mut out = String::with_capacity(n);
    while out.len() < n {
        let b = next_byte();
        if b < 250 {
            out.push((b'0' + b % 10) as char);
        }
    }
    out
}

/// HMAC-SHA256 hasher for short codes. `Debug` is redacted; the pepper is zeroized on drop.
///
/// The pepper MUST be at least 32 random bytes and stored outside the database
/// (e.g. in a secrets manager).
#[derive(Clone)]
pub struct CodeHasher {
    pepper: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for CodeHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CodeHasher(***)")
    }
}

impl CodeHasher {
    /// The pepper must be >= 32 random bytes, kept outside the database.
    /// Limiting attempts and expiring codes is the caller's responsibility.
    pub fn new(pepper: impl Into<Vec<u8>>) -> Self {
        Self {
            pepper: Zeroizing::new(pepper.into()),
        }
    }

    fn mac(&self, code: &str) -> Option<Vec<u8>> {
        let key = PKey::hmac(&self.pepper).ok()?;
        let mut signer = Signer::new(MessageDigest::sha256(), &key).ok()?;
        signer.update(code.as_bytes()).ok()?;
        signer.sign_to_vec().ok()
    }

    /// `hex(HMAC-SHA256(pepper, code))`. Returns an empty string only if OpenSSL fails
    /// (which can never match a stored hash in `verify`).
    pub fn hash(&self, code: &str) -> String {
        self.mac(code).map(|m| hex_encode(&m)).unwrap_or_default()
    }

    /// Constant-time comparison; malformed or wrong-sized `stored_hash` yields `false`.
    pub fn verify(&self, code: &str, stored_hash: &str) -> bool {
        let Some(stored) = hex_decode(stored_hash) else {
            return false;
        };
        if stored.len() != HMAC_SHA256_LEN {
            return false;
        }
        match self.mac(code) {
            Some(expected) if expected.len() == stored.len() => {
                openssl::memcmp::eq(&expected, &stored)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::util::random_bytes;

    fn random_pepper() -> Vec<u8> {
        random_bytes(32).unwrap()
    }

    #[test]
    fn hmac_known_vectors_rfc4231() {
        // Case 1: key = 0x0b * 20, data = "Hi There"
        let h = CodeHasher::new(vec![0x0b; 20]);
        assert_eq!(
            h.hash("Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // Case 2: key = "Jefe", data = "what do ya want for nothing?"
        let h = CodeHasher::new(b"Jefe".to_vec());
        assert_eq!(
            h.hash("what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hash_is_deterministic_and_keyed() {
        let a = CodeHasher::new(random_pepper());
        let b = CodeHasher::new(random_pepper());
        assert_eq!(a.hash("123456"), a.hash("123456"));
        assert_ne!(a.hash("123456"), b.hash("123456"));
        assert_ne!(a.hash("123456"), a.hash("654321"));
        assert_eq!(a.hash("123456").len(), 64);
    }

    #[test]
    fn verify_cases() {
        let h = CodeHasher::new(random_pepper());
        let stored = h.hash("123456");
        assert!(h.verify("123456", &stored));
        assert!(h.verify("123456", &stored.to_uppercase()));
        assert!(!h.verify("123457", &stored));
        assert!(!h.verify("123456", "zz"));
        assert!(!h.verify("123456", ""));
        assert!(!h.verify("123456", "abcd"));
        assert!(!h.verify("123456", &format!("{stored}00")));
        assert!(!h.verify("123456", &stored[..62]));
        let other = CodeHasher::new(random_pepper());
        assert!(!other.verify("123456", &stored));
    }

    #[test]
    fn rejection_sampling_discards_biased_bytes() {
        let mut seq = vec![255u8, 250, 3].into_iter();
        assert_eq!(digits_from_source(1, || seq.next().unwrap()), "3");
    }

    #[test]
    fn each_digit_has_exactly_25_preimages() {
        let mut counts = [0usize; 10];
        for b in 0..=255u8 {
            let mut once = Some(b);
            // Feed the byte, then a known good byte to terminate when it is discarded.
            let d = digits_from_source(1, || once.take().unwrap_or(0));
            if b < 250 {
                counts[(d.as_bytes()[0] - b'0') as usize] += 1;
            }
        }
        assert_eq!(counts, [25; 10]);
    }

    #[test]
    fn generated_codes_are_digits_only() {
        for _ in 0..10_000 {
            let c = generate_numeric_code(6);
            assert_eq!(c.len(), 6);
            assert!(c.bytes().all(|b| b.is_ascii_digit()));
        }
        assert_eq!(generate_numeric_code(0), "");
        assert_eq!(generate_numeric_code(40).len(), 40);
    }

    #[test]
    fn leading_zeros_are_kept() {
        let mut seq = vec![0u8, 0, 7].into_iter();
        assert_eq!(digits_from_source(3, || seq.next().unwrap()), "007");
    }

    #[test]
    fn debug_does_not_leak_pepper() {
        let marker = b"PEPPER-marker-0123456789-abcdefgh".to_vec();
        let h = CodeHasher::new(marker);
        let dbg = format!("{h:?}");
        assert!(!dbg.contains("PEPPER-marker"));
        assert_eq!(dbg, "CodeHasher(***)");
    }
}
