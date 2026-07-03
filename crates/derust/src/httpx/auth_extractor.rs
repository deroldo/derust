use axum::extract::Request;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{decode, decode_header, DecodingKey, TokenData, Validation};
use protect_endpoints_core::authorities::AuthDetails;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

use super::protect_endpoints_core::AuthoritiesClaims;

/// JWT authentication failure. All variants result in `401 Unauthorized` at the
/// HTTP layer, but are kept distinct to make debugging straightforward.
///
/// The error is inserted into the request extensions, so handlers or middlewares
/// can inspect it via `Extension<JwtAuthError>` when needed.
#[derive(Clone, Debug)]
pub enum JwtAuthError {
    /// The token has no `kid` header and the keystore has no default key.
    MissingKid,
    /// The token `kid` header does not match any key in the keystore.
    UnknownKid(String),
    /// The token is malformed, expired, or its signature does not match the
    /// selected key.
    InvalidToken(Arc<jsonwebtoken::errors::Error>),
}

impl std::fmt::Display for JwtAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKid => {
                write!(f, "token has no kid header and keystore has no default key")
            }
            Self::UnknownKid(kid) => write!(f, "token kid {kid:?} not found in keystore"),
            Self::InvalidToken(error) => write!(f, "invalid token: {error}"),
        }
    }
}

impl std::error::Error for JwtAuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidToken(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

/// Set of JWT decoding keys indexed by `kid`, with an optional fallback key
/// for tokens without a `kid` header (see [`JwtKeystore::with_fallback`]).
///
/// A token is always validated against a single key, selected by the `kid`
/// header of the token — keys are never tried in sequence.
#[derive(Clone)]
pub struct JwtKeystore {
    keys: HashMap<String, DecodingKey>,
    default: Option<DecodingKey>,
}

impl JwtKeystore {
    /// Single-key keystore. The key is the explicit default: tokens are
    /// validated against it regardless of carrying a `kid` header or not.
    ///
    /// This is the behaviour of [`AuthoritiesExtractor::new`].
    pub fn single(key: DecodingKey) -> Self {
        Self {
            keys: HashMap::new(),
            default: Some(key),
        }
    }

    /// Multi-key keystore. Tokens must carry a `kid` header matching one of
    /// the entries; there is no fallback key.
    pub fn new(keys: impl IntoIterator<Item = (String, DecodingKey)>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            default: None,
        }
    }

    /// Multi-key keystore with a fallback key for tokens **without** a `kid`
    /// header, typically legacy tokens issued before `kid` headers were
    /// adopted.
    ///
    /// Tokens carrying a `kid` are validated only against the matching entry:
    /// an unknown `kid` is rejected, never routed to the fallback.
    pub fn with_fallback(
        keys: impl IntoIterator<Item = (String, DecodingKey)>,
        fallback: DecodingKey,
    ) -> Self {
        Self {
            keys: keys.into_iter().collect(),
            default: Some(fallback),
        }
    }

    /// Builds a keystore from declarative configuration, typically loaded with
    /// `envx::load_app_config`. See [`JwtKeystoreConfig`].
    pub fn from_config(config: &JwtKeystoreConfig) -> Result<Self, JwtKeystoreConfigError> {
        let mut keys = HashMap::with_capacity(config.keys.len());
        let mut default = None;

        for key_config in config.keys.values() {
            let key = key_config.decoding_key()?;

            if key_config.fallback.unwrap_or(false) {
                if default.is_some() {
                    return Err(JwtKeystoreConfigError::MultipleFallbackKeys);
                }
                default = Some(key.clone());
            }

            if keys.insert(key_config.kid.clone(), key).is_some() {
                return Err(JwtKeystoreConfigError::DuplicatedKid(
                    key_config.kid.clone(),
                ));
            }
        }

        Ok(Self { keys, default })
    }

    /// Selects the decoding key for a token given its `kid` header.
    pub fn resolve(&self, kid: Option<&str>) -> Result<&DecodingKey, JwtAuthError> {
        match kid {
            Some(kid) => {
                if let Some(key) = self.keys.get(kid) {
                    return Ok(key);
                }
                // Single-key mode: the default validates every token, with or
                // without `kid`. On multi-key keystores an unknown `kid` is
                // rejected, never routed to the fallback.
                if self.keys.is_empty() {
                    if let Some(default) = &self.default {
                        return Ok(default);
                    }
                }
                Err(JwtAuthError::UnknownKid(kid.to_string()))
            }
            None => self.default.as_ref().ok_or(JwtAuthError::MissingKid),
        }
    }
}

/// Declarative configuration for a [`JwtKeystore`], loadable from env vars via
/// `envx::load_app_config`.
///
/// The map key is a free label (env loading does not preserve its case); the
/// token `kid` is matched against the [`JwtKeyConfig::kid`] field, which keeps
/// its exact case.
///
/// ```text
/// APP__JWT__KEYS__K2025__KID=key-2025
/// APP__JWT__KEYS__K2025__FORMAT=ec_pem
/// APP__JWT__KEYS__K2025__KEY=-----BEGIN PUBLIC KEY-----...
/// APP__JWT__KEYS__K2025__FALLBACK=true
/// ```
///
/// At most one key may set `fallback = true`; besides being addressable by its
/// `kid`, it also validates tokens without a `kid` header.
#[derive(Clone, Debug, Deserialize)]
pub struct JwtKeystoreConfig {
    pub keys: HashMap<String, JwtKeyConfig>,
}

/// A single `kid` + key pair. See [`JwtKeystoreConfig`].
#[derive(Clone, Debug, Deserialize)]
pub struct JwtKeyConfig {
    pub kid: String,
    pub format: JwtKeyFormat,
    pub key: String,
    pub fallback: Option<bool>,
}

impl JwtKeyConfig {
    fn decoding_key(&self) -> Result<DecodingKey, JwtKeystoreConfigError> {
        let result = match self.format {
            JwtKeyFormat::Secret => Ok(DecodingKey::from_secret(self.key.as_bytes())),
            JwtKeyFormat::Base64Secret => DecodingKey::from_base64_secret(&self.key),
            JwtKeyFormat::RsaPem => DecodingKey::from_rsa_pem(self.key.as_bytes()),
            JwtKeyFormat::EcPem => DecodingKey::from_ec_pem(self.key.as_bytes()),
            JwtKeyFormat::EdPem => DecodingKey::from_ed_pem(self.key.as_bytes()),
        };

        result.map_err(|source| JwtKeystoreConfigError::InvalidKey {
            kid: self.kid.clone(),
            source,
        })
    }
}

/// How the [`JwtKeyConfig::key`] value should be interpreted.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JwtKeyFormat {
    /// Raw HMAC secret
    Secret,
    /// Base64-encoded HMAC secret
    Base64Secret,
    /// RSA public key in PEM format
    RsaPem,
    /// EC public key in PEM format
    EcPem,
    /// Ed25519 public key in PEM format
    EdPem,
}

/// Failure building a [`JwtKeystore`] from a [`JwtKeystoreConfig`].
#[derive(Debug)]
pub enum JwtKeystoreConfigError {
    InvalidKey {
        kid: String,
        source: jsonwebtoken::errors::Error,
    },
    DuplicatedKid(String),
    MultipleFallbackKeys,
}

impl std::fmt::Display for JwtKeystoreConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey { kid, source } => {
                write!(f, "invalid key for kid {kid:?}: {source}")
            }
            Self::DuplicatedKid(kid) => write!(f, "duplicated kid {kid:?}"),
            Self::MultipleFallbackKeys => write!(f, "more than one key marked as fallback"),
        }
    }
}

impl std::error::Error for JwtKeystoreConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidKey { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn validate_token<C: DeserializeOwned>(
    token: &str,
    keystore: &JwtKeystore,
    validation: &Validation,
) -> Result<TokenData<C>, JwtAuthError> {
    let header =
        decode_header(token).map_err(|error| JwtAuthError::InvalidToken(Arc::new(error)))?;
    let key = keystore.resolve(header.kid.as_deref())?;
    decode::<C>(token, key, validation).map_err(|error| JwtAuthError::InvalidToken(Arc::new(error)))
}

#[derive(Clone)]
pub struct AuthoritiesExtractor<C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    keystore: Arc<JwtKeystore>,
    validation: Arc<Validation>,
    _phantom: PhantomData<fn() -> C>,
}

impl<C> AuthoritiesExtractor<C>
where
    C: Clone + DeserializeOwned + AuthoritiesClaims + Send + Sync + 'static,
{
    /// Single-key extractor. Equivalent to
    /// `with_keystore(JwtKeystore::single(decoding_key), validation)`: the key
    /// is used for every token, whether or not it carries a `kid` header.
    pub fn new(decoding_key: DecodingKey, validation: Validation) -> Self {
        Self::with_keystore(JwtKeystore::single(decoding_key), validation)
    }

    /// Multi-key extractor. The token `kid` header selects the decoding key
    /// from the keystore. See [`JwtKeystore`].
    pub fn with_keystore(keystore: JwtKeystore, validation: Validation) -> Self {
        Self {
            keystore: Arc::new(keystore),
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
            keystore: self.keystore.clone(),
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
    keystore: Arc<JwtKeystore>,
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
        let keystore = self.keystore.clone();
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
                match validate_token::<C>(&token, &keystore, &validation) {
                    Ok(token_data) => {
                        let roles: HashSet<String> =
                            token_data.claims.roles().into_iter().collect();
                        req.extensions_mut().insert(AuthDetails::new(roles));
                        req.extensions_mut().insert(token_data.claims);
                    }
                    Err(error) => {
                        warn!("JWT authentication failed: {error}");
                        req.extensions_mut().insert(error);
                    }
                }
            }

            inner.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::StatusCode;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;
    use std::convert::Infallible;
    use tower::service_fn;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct TestClaims {
        sub: String,
        exp: i64,
    }

    impl AuthoritiesClaims for TestClaims {
        fn roles(&self) -> Vec<String> {
            vec!["USER".to_string()]
        }
    }

    fn claims(sub: &str) -> TestClaims {
        TestClaims {
            sub: sub.to_string(),
            exp: 4102444800, // 2100-01-01
        }
    }

    fn token(kid: Option<&str>, secret: &[u8], sub: &str) -> String {
        let mut header = Header::new(Algorithm::HS256);
        header.kid = kid.map(str::to_owned);
        encode(&header, &claims(sub), &EncodingKey::from_secret(secret)).unwrap()
    }

    fn validation() -> Validation {
        Validation::new(Algorithm::HS256)
    }

    fn multi_keystore() -> JwtKeystore {
        JwtKeystore::new([
            ("key-1".to_string(), DecodingKey::from_secret(b"secret-1")),
            ("key-2".to_string(), DecodingKey::from_secret(b"secret-2")),
        ])
    }

    #[test]
    fn should_validate_token_with_known_kid() {
        let token = token(Some("key-1"), b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &multi_keystore(), &validation());

        assert_eq!(result.unwrap().claims.sub, "alice");
    }

    #[test]
    fn should_reject_token_with_unknown_kid() {
        let token = token(Some("key-3"), b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &multi_keystore(), &validation());

        assert!(matches!(result, Err(JwtAuthError::UnknownKid(kid)) if kid == "key-3"));
    }

    #[test]
    fn should_validate_token_without_kid_using_single_key_default() {
        let keystore = JwtKeystore::single(DecodingKey::from_secret(b"secret-1"));
        let token = token(None, b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &keystore, &validation());

        assert_eq!(result.unwrap().claims.sub, "alice");
    }

    #[test]
    fn should_reject_token_without_kid_when_there_is_no_default() {
        let token = token(None, b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &multi_keystore(), &validation());

        assert!(matches!(result, Err(JwtAuthError::MissingKid)));
    }

    fn fallback_keystore() -> JwtKeystore {
        JwtKeystore::with_fallback(
            [("key-1".to_string(), DecodingKey::from_secret(b"secret-1"))],
            DecodingKey::from_secret(b"legacy-secret"),
        )
    }

    #[test]
    fn should_use_fallback_for_token_without_kid() {
        let token = token(None, b"legacy-secret", "alice");

        let result = validate_token::<TestClaims>(&token, &fallback_keystore(), &validation());

        assert_eq!(result.unwrap().claims.sub, "alice");
    }

    #[test]
    fn should_validate_known_kid_on_fallback_keystore() {
        let token = token(Some("key-1"), b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &fallback_keystore(), &validation());

        assert_eq!(result.unwrap().claims.sub, "alice");
    }

    #[test]
    fn should_reject_unknown_kid_even_with_fallback() {
        let token = token(Some("key-9"), b"legacy-secret", "alice");

        let result = validate_token::<TestClaims>(&token, &fallback_keystore(), &validation());

        assert!(matches!(result, Err(JwtAuthError::UnknownKid(kid)) if kid == "key-9"));
    }

    #[test]
    fn should_reject_token_without_kid_signed_by_other_key_on_fallback_keystore() {
        let token = token(None, b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &fallback_keystore(), &validation());

        assert!(matches!(result, Err(JwtAuthError::InvalidToken(_))));
    }

    #[test]
    fn should_reject_token_with_invalid_signature_even_with_known_kid() {
        let token = token(Some("key-1"), b"wrong-secret", "alice");

        let result = validate_token::<TestClaims>(&token, &multi_keystore(), &validation());

        assert!(matches!(result, Err(JwtAuthError::InvalidToken(_))));
    }

    #[test]
    fn should_validate_each_token_only_against_its_own_key() {
        let keystore = multi_keystore();

        let token_1 = token(Some("key-1"), b"secret-1", "alice");
        let token_2 = token(Some("key-2"), b"secret-2", "bob");
        let crossed = token(Some("key-1"), b"secret-2", "eve");

        let result_1 = validate_token::<TestClaims>(&token_1, &keystore, &validation());
        let result_2 = validate_token::<TestClaims>(&token_2, &keystore, &validation());
        let result_crossed = validate_token::<TestClaims>(&crossed, &keystore, &validation());

        assert_eq!(result_1.unwrap().claims.sub, "alice");
        assert_eq!(result_2.unwrap().claims.sub, "bob");
        assert!(matches!(result_crossed, Err(JwtAuthError::InvalidToken(_))));
    }

    #[test]
    fn should_ignore_kid_on_single_key_keystore() {
        let keystore = JwtKeystore::single(DecodingKey::from_secret(b"secret-1"));
        let token = token(Some("any-kid"), b"secret-1", "alice");

        let result = validate_token::<TestClaims>(&token, &keystore, &validation());

        assert_eq!(result.unwrap().claims.sub, "alice");
    }

    #[test]
    fn should_build_keystore_from_config() {
        let config = JwtKeystoreConfig {
            keys: HashMap::from([
                (
                    "k1".to_string(),
                    JwtKeyConfig {
                        kid: "key-1".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "secret-1".to_string(),
                        fallback: None,
                    },
                ),
                (
                    "k2".to_string(),
                    JwtKeyConfig {
                        kid: "key-2".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "secret-2".to_string(),
                        fallback: None,
                    },
                ),
            ]),
        };

        let keystore = JwtKeystore::from_config(&config).unwrap();
        let token = token(Some("key-2"), b"secret-2", "bob");

        let result = validate_token::<TestClaims>(&token, &keystore, &validation());

        assert_eq!(result.unwrap().claims.sub, "bob");
    }

    #[test]
    fn should_build_keystore_from_config_with_fallback_key() {
        let config = JwtKeystoreConfig {
            keys: HashMap::from([
                (
                    "k1".to_string(),
                    JwtKeyConfig {
                        kid: "key-1".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "secret-1".to_string(),
                        fallback: None,
                    },
                ),
                (
                    "legacy".to_string(),
                    JwtKeyConfig {
                        kid: "key-legacy".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "legacy-secret".to_string(),
                        fallback: Some(true),
                    },
                ),
            ]),
        };

        let keystore = JwtKeystore::from_config(&config).unwrap();
        let no_kid_token = token(None, b"legacy-secret", "alice");
        let kid_token = token(Some("key-legacy"), b"legacy-secret", "bob");

        let no_kid_result = validate_token::<TestClaims>(&no_kid_token, &keystore, &validation());
        let kid_result = validate_token::<TestClaims>(&kid_token, &keystore, &validation());

        assert_eq!(no_kid_result.unwrap().claims.sub, "alice");
        assert_eq!(kid_result.unwrap().claims.sub, "bob");
    }

    #[test]
    fn should_fail_building_keystore_with_multiple_fallback_keys() {
        let config = JwtKeystoreConfig {
            keys: HashMap::from([
                (
                    "k1".to_string(),
                    JwtKeyConfig {
                        kid: "key-1".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "secret-1".to_string(),
                        fallback: Some(true),
                    },
                ),
                (
                    "k2".to_string(),
                    JwtKeyConfig {
                        kid: "key-2".to_string(),
                        format: JwtKeyFormat::Secret,
                        key: "secret-2".to_string(),
                        fallback: Some(true),
                    },
                ),
            ]),
        };

        let result = JwtKeystore::from_config(&config);

        assert!(matches!(
            result,
            Err(JwtKeystoreConfigError::MultipleFallbackKeys)
        ));
    }

    #[test]
    fn should_fail_building_keystore_with_duplicated_kid() {
        let key_config = JwtKeyConfig {
            kid: "key-1".to_string(),
            format: JwtKeyFormat::Secret,
            key: "secret-1".to_string(),
            fallback: None,
        };
        let config = JwtKeystoreConfig {
            keys: HashMap::from([
                ("k1".to_string(), key_config.clone()),
                ("k2".to_string(), key_config),
            ]),
        };

        let result = JwtKeystore::from_config(&config);

        assert!(matches!(
            result,
            Err(JwtKeystoreConfigError::DuplicatedKid(kid)) if kid == "key-1"
        ));
    }

    async fn call_service(keystore: JwtKeystore, authorization: Option<String>) -> Response {
        let layer = AuthoritiesExtractor::<TestClaims>::with_keystore(keystore, validation());

        let mut service = layer.layer(service_fn(|req: Request| async move {
            let status = if req.extensions().get::<AuthDetails<String>>().is_some()
                && req.extensions().get::<TestClaims>().is_some()
            {
                StatusCode::OK
            } else if req.extensions().get::<JwtAuthError>().is_some() {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::NO_CONTENT
            };
            Ok::<_, Infallible>(status.into_response())
        }));

        let mut builder = Request::builder().uri("/");
        if let Some(authorization) = authorization {
            builder = builder.header(header::AUTHORIZATION, authorization);
        }
        let request = builder.body(Body::empty()).unwrap();

        service.call(request).await.unwrap()
    }

    #[tokio::test]
    async fn should_insert_auth_details_and_claims_on_valid_token() {
        let token = token(Some("key-1"), b"secret-1", "alice");

        let response = call_service(multi_keystore(), Some(format!("Bearer {token}"))).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_insert_auth_error_on_unknown_kid() {
        let token = token(Some("key-3"), b"secret-1", "alice");

        let response = call_service(multi_keystore(), Some(format!("Bearer {token}"))).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_not_insert_anything_without_authorization_header() {
        let response = call_service(multi_keystore(), None).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
