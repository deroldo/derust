use thiserror::Error;

/// Errors raised while loading or validating cryptographic keys.
/// Messages are fixed and never include key material.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeyError {
    #[error("invalid PEM")]
    InvalidPem,
    #[error("private key is not PKCS#8")]
    NotPkcs8,
    #[error("public key is not SPKI")]
    NotSpki,
    #[error("key is not an EC P-256 key")]
    NotEcP256,
    #[error("duplicate kid in key set")]
    DuplicateKid,
    #[error("kid must not be empty")]
    EmptyKid,
    #[error("failed to read key file")]
    Io,
    #[error("failed to load key from source")]
    Source,
}

/// Errors raised while signing or verifying JWTs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum JwtError {
    #[error("malformed token")]
    Malformed,
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,
    #[error("unknown kid")]
    UnknownKid,
    #[error("bad signature")]
    BadSignature,
    #[error("token expired")]
    Expired,
    #[error("token not yet valid")]
    NotYetValid,
    #[error("wrong issuer")]
    WrongIssuer,
    #[error("wrong audience")]
    WrongAudience,
    #[error("wrong token type")]
    WrongTokenType,
    #[error("reserved claim")]
    ReservedClaim,
    #[error("invalid claims")]
    InvalidClaims,
    #[error("invalid configuration")]
    InvalidConfig,
    #[error("signing failed")]
    SigningFailed,
}

/// Errors raised while sealing or opening JWE tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum JweError {
    #[error("malformed JWE")]
    Malformed,
    #[error("decryption failed")]
    DecryptFailed,
    #[error("invalid key")]
    InvalidKey,
    #[error("sealing failed")]
    SealFailed,
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,
}

/// Errors raised by the password hasher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PasswordError {
    #[error("invalid argon2 parameters")]
    InvalidParams,
    #[error("hashing failed")]
    HashFailed,
    #[error("invalid hash")]
    InvalidHash,
    #[error("unsupported algorithm")]
    UnsupportedAlgorithm,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn display_is_fixed_per_variant() {
        assert_eq!(JwtError::Malformed.to_string(), "malformed token");
        assert_eq!(KeyError::InvalidPem.to_string(), "invalid PEM");
        assert_eq!(JweError::DecryptFailed.to_string(), "decryption failed");
        assert_eq!(PasswordError::InvalidHash.to_string(), "invalid hash");
    }

    #[test]
    fn debug_contains_only_variant_names() {
        assert_eq!(format!("{:?}", JwtError::BadSignature), "BadSignature");
        assert_eq!(format!("{:?}", KeyError::Source), "Source");
    }
}
