mod auth_extractor;
mod axum;
mod config;
mod context;

pub mod protect_endpoints_core {
    pub use super::auth_extractor::{
        AuthoritiesExtractor, JwtAuthError, JwtKeyConfig, JwtKeyFormat, JwtKeystore,
        JwtKeystoreConfig, JwtKeystoreConfigError,
    };
    pub use ::protect_endpoints_core::*;
    pub use jsonwebtoken::{Algorithm, DecodingKey, Validation};

    pub trait AuthoritiesClaims {
        fn roles(&self) -> Vec<String>;
    }
}

pub(crate) mod extension;
mod middlewares;

mod health;

mod request;
mod response;
mod server;
mod tags;

#[cfg(feature = "prometheus")]
mod prometheus;

#[cfg(feature = "growthbook")]
pub use growthbook_rust_sdk::client::*;

pub use config::*;
pub use context::*;
pub use error::*;
pub use request::json_request::*;
pub use response::json::*;
pub use response::*;
pub use server::*;
pub use tags::*;
