//! JWT/JOSE, encryption and hashing helpers (`cryptox` feature).
//!
//! Design notes:
//! - Everything is an instance value: there is no global/static state, so several
//!   independent configurations (e.g. one `TokenIssuer` per channel) can coexist in
//!   the same process.
//! - `josekit` is only used inside the private `jose` module; none of its types
//!   appear in the public API.
//! - Error messages are fixed per variant and never carry tokens, keys, passwords
//!   or codes.
//!
//! # Issuer example (identity-service)
//!
//! One [`TokenIssuer`] per channel: issuer, audience and TTLs differ and coexist in the process.
//!
//! ```
//! use derust::cryptox::{
//!     generate_ec_p256_keypair, generate_numeric_code, Argon2Params, CodeHasher, JweOpener,
//!     JweSealer, PasswordHasher, Sealer, SigningKey, TokenIssuer, TokenIssuerSettings,
//!     VerifyingKeySet,
//! };
//! use serde::Serialize;
//! use std::time::Duration;
//!
//! #[derive(Serialize)]
//! struct MyClaims {
//!     role: String,
//! }
//!
//! fn issuer_for(channel: &str, ttl: (u64, u64)) -> Result<(TokenIssuer, VerifyingKeySet), Box<dyn std::error::Error>> {
//!     // In a service the PEMs come from FileKeySource / Secrets Manager (see `KeySourceConfig`).
//!     let (access_pem, _) = generate_ec_p256_keypair()?;
//!     let (refresh_pem, _) = generate_ec_p256_keypair()?;
//!     let access = SigningKey::from_pem(access_pem, format!("{channel}-access-1"))?;
//!     let refresh = SigningKey::from_pem(refresh_pem, format!("{channel}-refresh-1"))?;
//!     let jwks_keys = VerifyingKeySet::new(vec![access.verifying_key()])?;
//!     let issuer = TokenIssuer::new(
//!         access,
//!         refresh,
//!         TokenIssuerSettings {
//!             issuer: format!("https://id.example/{channel}"),
//!             audience: format!("bff-{channel}"),
//!             access_ttl: Duration::from_secs(ttl.0),
//!             refresh_ttl: Duration::from_secs(ttl.1),
//!         },
//!     )?;
//!     Ok((issuer, jwks_keys))
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let (web, web_keys) = issuer_for("web", (900, 30 * 86_400))?;
//!     let (mobile, _) = issuer_for("mobile", (1_800, 90 * 86_400))?;
//!
//!     let access_token = web.access_signer().sign("user-1", MyClaims { role: "admin".into() })?;
//!     let refresh_token = mobile.refresh_signer().sign("user-1", ())?;
//!     assert_ne!(access_token, refresh_token);
//!     let jwks = web_keys.to_jwks(); // publish at /.well-known/jwks.json
//!     assert!(jwks["keys"][0].get("d").is_none());
//!
//!     // JWE: seal to the recipient's public key, only the private key opens it.
//!     let (jwe_private, jwe_public) = generate_ec_p256_keypair()?;
//!     let sealed = JweSealer::new(&jwe_public)?.seal(b"sensitive payload")?;
//!     assert_eq!(JweOpener::new(&jwe_private)?.open(&sealed)?, b"sensitive payload");
//!
//!     // Passwords: Argon2id (CPU-bound: use spawn_blocking in async code).
//!     let hasher = PasswordHasher::new(Argon2Params::default())?;
//!     let phc = hasher.hash("correct horse battery staple")?;
//!     assert!(hasher.verify("correct horse battery staple", &phc)?);
//!     assert!(!hasher.needs_rehash(&phc));
//!
//!     // Short codes (OTP): random digits, stored as HMAC-SHA256 with a secret pepper.
//!     let pepper = b"0123456789abcdef0123456789abcdef".to_vec(); // load >= 32 random bytes from a secret store
//!     let codes = CodeHasher::new(pepper);
//!     let code = generate_numeric_code(6);
//!     let stored = codes.hash(&code);
//!     assert!(codes.verify(&code, &stored));
//!     Ok(())
//! }
//! ```
//!
//! The validator side (BFF) is documented in [`http`] (needs `http_server`).

mod code;
mod config;
mod error;
#[cfg(feature = "http_server")]
pub mod http;
mod jose;
mod jwe;
mod jwt;
mod keys;
mod password;
mod util;

pub use code::{generate_numeric_code, CodeHasher};
#[cfg(feature = "aws")]
pub use config::SecretsManagerKeySource;
pub use config::{
    EnvKeySource, FileKeySource, JwtVerifierConfig, KeySource, KeySourceConfig, KeySourceResolver,
    SigningKeyConfig, TokenIssuerConfig, VerifyingKeyConfig,
};
pub use error::{ConfigError, JweError, JwtError, KeyError, PasswordError};
#[cfg(feature = "http_server")]
pub use http::{AuthConfig, AuthLayer, Authenticated, RequireClaims};
pub use jwe::{JweOpener, JweSealer, Sealer};
pub use jwt::{
    JwtSigner, JwtSignerSettings, JwtVerifier, TokenIssuer, TokenIssuerSettings, TokenType,
    VerifiedJwt,
};
pub use keys::{generate_ec_p256_keypair, SigningKey, VerifyingKey, VerifyingKeySet};
pub use password::{Argon2Params, PasswordHasher};
