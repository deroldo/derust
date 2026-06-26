use axum::extract::Request;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{DecodingKey, Validation, decode};
use protect_endpoints_core::authorities::AuthDetails;
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::protect_endpoints_core::AuthoritiesClaims;

#[derive(Clone)]
pub struct AuthoritiesExtractor<C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    decoding_key: Arc<DecodingKey>,
    validation: Arc<Validation>,
    _phantom: PhantomData<fn() -> C>,
}

impl<C> AuthoritiesExtractor<C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    pub fn new(decoding_key: DecodingKey, validation: Validation) -> Self {
        Self {
            decoding_key: Arc::new(decoding_key),
            validation: Arc::new(validation),
            _phantom: PhantomData,
        }
    }

    pub async fn grants_extractor(req: &mut Request) -> Result<HashSet<String>, Response> {
        req.extensions()
            .get::<AuthDetails<String>>()
            .map(|d| d.authorities.iter().cloned().collect())
            .ok_or_else(|| axum::http::StatusCode::UNAUTHORIZED.into_response())
    }
}

impl<S, C> Layer<S> for AuthoritiesExtractor<C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    type Service = AuthoritiesExtractorService<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthoritiesExtractorService {
            inner,
            decoding_key: self.decoding_key.clone(),
            validation: self.validation.clone(),
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct AuthoritiesExtractorService<S, C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    inner: S,
    decoding_key: Arc<DecodingKey>,
    validation: Arc<Validation>,
    _phantom: PhantomData<fn() -> C>,
}

impl<S, C> Service<Request> for AuthoritiesExtractorService<S, C>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
    S::Error: Send,
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let decoding_key = self.decoding_key.clone();
        let validation = self.validation.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(str::to_owned);

            if let Some(token) = token {
                if let Ok(token_data) = decode::<C>(&token, &decoding_key, &validation) {
                    let roles: HashSet<String> = token_data.claims.roles().into_iter().collect();
                    req.extensions_mut().insert(AuthDetails::new(roles));
                    req.extensions_mut().insert(token_data.claims);
                }
            }

            inner.call(req).await
        })
    }
}
