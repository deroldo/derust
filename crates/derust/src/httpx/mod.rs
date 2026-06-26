mod axum;
mod auth_extractor;
mod config;
mod context;

pub mod protect_endpoints_core {
    pub use ::protect_endpoints_core::*;
    pub use jsonwebtoken::{Algorithm, DecodingKey, Validation};
    pub use super::auth_extractor::AuthoritiesExtractor;

    pub trait AuthoritiesClaims {
        fn roles(&self) -> Vec<String>;
    }
}

pub(crate) mod extension;
mod middlewares;

mod health;

mod response;
mod request;
mod server;
mod tags;

#[cfg(feature = "prometheus")]
mod prometheus;

#[cfg(feature = "growthbook")]
pub use growthbook_rust_sdk::client::*;

pub use config::*;
pub use context::*;
pub use error::*;
pub use response::*;
pub use response::json::*;
pub use request::json_request::*;
pub use server::*;
pub use tags::*;
