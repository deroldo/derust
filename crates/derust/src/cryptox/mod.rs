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

mod code;
mod config;
mod error;
#[cfg(feature = "http_server")]
mod http;
mod jose;
mod jwe;
mod jwt;
mod keys;
mod password;
mod util;

pub use error::{JweError, JwtError, KeyError, PasswordError};
pub use keys::{generate_ec_p256_keypair, SigningKey, VerifyingKey, VerifyingKeySet};
