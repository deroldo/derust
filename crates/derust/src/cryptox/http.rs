//! Axum integration (requires `http_server`): [`AuthConfig`], [`Authenticated`], [`RequireClaims`].
//!
//! The verification config reaches the extractor through a request extension set by
//! [`AuthConfig::layer`] (the orphan rule prevents a `FromRef<AppContext<S>>` impl).
//! Failures answer `401` with `WWW-Authenticate: Bearer` and an empty body, `403` when a
//! [`RequireClaims`] predicate refuses, and `500` when the config layer is missing. The reason
//! (variant name only, never the token) goes to `DerustTags` (`auth_failure`).
//!
//! Layer order: `.layer(auth.layer())` must come after (outside of) `route_layer`:
//!
//! ```no_run
//! use axum::{routing::get, Router};
//! use derust::cryptox::http::{AuthConfig, Authenticated, RequireClaims};
//! use derust::cryptox::{generate_ec_p256_keypair, VerifyingKey, VerifyingKeySet};
//! use serde::Deserialize;
//!
//! #[derive(Clone, Deserialize)]
//! struct Claims { role: String }
//!
//! async fn me(Authenticated(jwt): Authenticated<Claims>) -> String {
//!     jwt.subject().to_owned()
//! }
//!
//! # fn build() -> Result<Router, Box<dyn std::error::Error>> {
//! # let (_, public_pem) = generate_ec_p256_keypair()?;
//! let set = VerifyingKeySet::new(vec![VerifyingKey::from_pem(public_pem, "k1")?])?;
//! let auth = AuthConfig::new(set, "https://id.example", "bff")?;
//! let router: Router = Router::new()
//!     .route("/me", get(me))
//!     .route_layer(RequireClaims::<Claims>::new(|c| c.role == "admin"))
//!     .layer(auth.layer());
//! # Ok(router) }
//! ```

use crate::commonx::{DerustError, DerustTags};
use crate::cryptox::error::JwtError;
use crate::cryptox::jwt::{JwtVerifier, TokenType, VerifiedJwt};
use crate::cryptox::keys::VerifyingKeySet;
use axum::body::Body;
use axum::extract::{FromRequestParts, Request};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Layer, Service};

/// Config placed in request extensions by [`AuthLayer`]. Private on purpose.
#[derive(Clone)]
struct AuthState(Arc<JwtVerifier>);

/// Verification settings for access tokens (`iss` and `aud` are mandatory).
#[derive(Clone, Debug)]
pub struct AuthConfig {
    verifier: Arc<JwtVerifier>,
}

impl AuthConfig {
    /// Fails with `InvalidConfig` for empty `issuer`/`audience` or an empty key set.
    pub fn new(
        keys: VerifyingKeySet,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Result<Self, JwtError> {
        let verifier = JwtVerifier::new(keys, TokenType::Access, issuer, audience)?;
        Ok(Self {
            verifier: Arc::new(verifier),
        })
    }

    /// Clock-skew tolerance (default: zero).
    pub fn leeway(self, leeway: Duration) -> Self {
        let verifier = (*self.verifier).clone().leeway(leeway);
        Self {
            verifier: Arc::new(verifier),
        }
    }

    pub fn layer(&self) -> AuthLayer {
        AuthLayer {
            state: AuthState(self.verifier.clone()),
        }
    }
}

/// Tower layer that makes the [`AuthConfig`] available to [`Authenticated`] and [`RequireClaims`].
#[derive(Clone)]
pub struct AuthLayer {
    state: AuthState,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    state: AuthState,
}

impl<S, B> Service<Request<B>> for AuthService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        req.extensions_mut().insert(self.state.clone());
        self.inner.call(req)
    }
}

fn unauthorized(reason: &str) -> DerustError {
    DerustError::without_body(
        StatusCode::UNAUTHORIZED,
        "unauthorized".to_string(),
        DerustTags::from([("auth_failure", reason)]),
    )
    .with_headers(vec![(
        WWW_AUTHENTICATE.as_str().to_string(),
        "Bearer".to_string(),
    )])
}

fn forbidden() -> DerustError {
    DerustError::without_body(
        StatusCode::FORBIDDEN,
        "forbidden".to_string(),
        DerustTags::from([("auth_failure", "PredicateDenied")]),
    )
}

fn not_configured() -> DerustError {
    tracing::error!(
        "cryptox auth is not configured for this route: apply AuthConfig::layer() after (outside) the route_layer calls"
    );
    DerustError::without_body(
        StatusCode::INTERNAL_SERVER_ERROR,
        "auth not configured".to_string(),
        DerustTags::from([("auth_failure", "NotConfigured")]),
    )
}

/// `Authorization: Bearer <token>`: exactly one header, case-insensitive scheme, one space, non-empty token.
fn bearer_token(headers: &HeaderMap) -> Result<&str, DerustError> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next().ok_or_else(|| unauthorized("MissingToken"))?;
    if values.next().is_some() {
        return Err(unauthorized("MalformedHeader"));
    }
    let value = value
        .to_str()
        .map_err(|_| unauthorized("MalformedHeader"))?;
    let (scheme, token) = value
        .split_once(' ')
        .ok_or_else(|| unauthorized("MalformedHeader"))?;
    if !scheme.eq_ignore_ascii_case("bearer")
        || token.is_empty()
        || token.chars().any(char::is_whitespace)
    {
        return Err(unauthorized("MalformedHeader"));
    }
    Ok(token)
}

/// Reuses a `VerifiedJwt<C>` already in the extensions, otherwise verifies the bearer token.
fn resolve<C>(headers: &HeaderMap, extensions: &Extensions) -> Result<VerifiedJwt<C>, DerustError>
where
    C: DeserializeOwned + Clone + Send + Sync + 'static,
{
    if let Some(done) = extensions.get::<VerifiedJwt<C>>() {
        return Ok(done.clone());
    }
    let state = extensions.get::<AuthState>().ok_or_else(not_configured)?;
    let token = bearer_token(headers)?;
    state
        .0
        .verify::<C>(token)
        .map_err(|e| unauthorized(&format!("{e:?}")))
}

/// Extractor for a verified access token. Reuses the result computed by [`RequireClaims`].
#[derive(Clone, Debug)]
pub struct Authenticated<C>(pub VerifiedJwt<C>);

impl<S, C> FromRequestParts<S> for Authenticated<C>
where
    S: Send + Sync,
    C: DeserializeOwned + Clone + Send + Sync + 'static,
{
    type Rejection = DerustError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        resolve::<C>(&parts.headers, &parts.extensions).map(Authenticated)
    }
}

type Predicate<C> = Arc<dyn Fn(&C) -> bool + Send + Sync>;

/// Layer that verifies the token and checks a predicate over the claims
/// (`401` on authentication failure, `403` when the predicate is false).
pub struct RequireClaims<C> {
    predicate: Predicate<C>,
}

impl<C> Clone for RequireClaims<C> {
    fn clone(&self) -> Self {
        Self {
            predicate: self.predicate.clone(),
        }
    }
}

impl<C> RequireClaims<C> {
    pub fn new(predicate: impl Fn(&C) -> bool + Send + Sync + 'static) -> Self {
        Self {
            predicate: Arc::new(predicate),
        }
    }
}

impl<S, C> Layer<S> for RequireClaims<C> {
    type Service = RequireClaimsService<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireClaimsService {
            inner,
            predicate: self.predicate.clone(),
            _claims: PhantomData,
        }
    }
}

pub struct RequireClaimsService<S, C> {
    inner: S,
    predicate: Predicate<C>,
    _claims: PhantomData<fn() -> C>,
}

impl<S: Clone, C> Clone for RequireClaimsService<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            predicate: self.predicate.clone(),
            _claims: PhantomData,
        }
    }
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

impl<S, C> Service<Request<Body>> for RequireClaimsService<S, C>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    C: DeserializeOwned + Clone + Send + Sync + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = BoxFuture<Result<Response<Body>, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let predicate = self.predicate.clone();
        Box::pin(async move {
            match resolve::<C>(req.headers(), req.extensions()) {
                Err(e) => Ok(e.into()),
                Ok(jwt) => {
                    if !predicate(jwt.claims()) {
                        return Ok(forbidden().into());
                    }
                    req.extensions_mut().insert(jwt);
                    inner.call(req).await
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::jwt::{JwtSigner, JwtSignerSettings};
    use crate::cryptox::keys::{generate_ec_p256_keypair, SigningKey};
    use axum::routing::get;
    use axum::Router;
    use serde::{Deserialize, Serialize};
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use tower::service_fn;
    use tower::ServiceExt;

    const ISS: &str = "https://id.example";
    const AUD: &str = "bff";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Claims {
        role: String,
    }

    struct Fixture {
        sk: SigningKey,
        auth: AuthConfig,
    }

    fn fixture() -> Fixture {
        let (private, _) = generate_ec_p256_keypair().unwrap();
        let sk = SigningKey::from_pem(private, "k1").unwrap();
        let set = VerifyingKeySet::new(vec![sk.verifying_key()]).unwrap();
        Fixture {
            sk,
            auth: AuthConfig::new(set, ISS, AUD).unwrap(),
        }
    }

    fn signer(sk: &SigningKey, tt: TokenType, iss: &str, aud: &str, ttl: u64) -> JwtSigner {
        JwtSigner::new(
            sk.clone(),
            tt,
            JwtSignerSettings {
                issuer: iss.into(),
                audience: aud.into(),
                ttl: Duration::from_secs(ttl),
            },
        )
        .unwrap()
    }

    fn token(sk: &SigningKey, role: &str) -> String {
        signer(sk, TokenType::Access, ISS, AUD, 300)
            .sign("user-1", Claims { role: role.into() })
            .unwrap()
    }

    async fn me(Authenticated(jwt): Authenticated<Claims>) -> String {
        format!("{}:{}", jwt.subject(), jwt.claims().role)
    }

    fn plain_router(auth: &AuthConfig) -> Router {
        Router::new().route("/me", get(me)).layer(auth.layer())
    }

    fn admin_router(auth: &AuthConfig, calls: Arc<AtomicUsize>) -> Router {
        let handler = move |Authenticated(jwt): Authenticated<Claims>| {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                jwt.subject().to_owned()
            }
        };
        Router::new()
            .route("/admin", get(handler))
            .route_layer(RequireClaims::<Claims>::new(|c| c.role == "admin"))
            .layer(auth.layer())
    }

    async fn call(router: Router, header: Option<&str>) -> (StatusCode, HeaderMap, String) {
        let mut builder = axum::http::Request::builder().uri("/me");
        if let Some(h) = header {
            builder = builder.header(AUTHORIZATION, h);
        }
        send(router, builder.body(Body::empty()).unwrap()).await
    }

    async fn send(router: Router, req: Request<Body>) -> (StatusCode, HeaderMap, String) {
        let res = router.oneshot(req).await.unwrap();
        let (parts, body) = res.into_parts();
        let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
        (
            parts.status,
            parts.headers,
            String::from_utf8_lossy(&bytes).into_owned(),
        )
    }

    fn assert_generic_401(status: StatusCode, headers: &HeaderMap, body: &str, secret: &str) {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(headers.get(WWW_AUTHENTICATE).unwrap(), "Bearer");
        assert!(body.is_empty() || !body.contains(secret));
        let lowered = body.to_lowercase();
        for reason in ["expired", "signature", "issuer", "audience", "kid"] {
            assert!(!lowered.contains(reason), "{reason}");
        }
    }

    #[tokio::test]
    async fn valid_token_reaches_the_handler() {
        let f = fixture();
        let t = token(&f.sk, "admin");
        let (s, _, body) = call(plain_router(&f.auth), Some(&format!("Bearer {t}"))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body, "user-1:admin");
        let (s, _, _) = call(plain_router(&f.auth), Some(&format!("bEaReR {t}"))).await;
        assert_eq!(s, StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_requests_get_a_generic_401() {
        let f = fixture();
        let good = token(&f.sk, "admin");
        let other = fixture();
        let mut tampered = good.clone();
        tampered.push('x');
        let refresh = signer(&f.sk, TokenType::Refresh, ISS, AUD, 300)
            .sign(
                "u",
                Claims {
                    role: "admin".into(),
                },
            )
            .unwrap();
        let wrong_iss = signer(&f.sk, TokenType::Access, "other", AUD, 300)
            .sign(
                "u",
                Claims {
                    role: "admin".into(),
                },
            )
            .unwrap();
        let wrong_aud = signer(&f.sk, TokenType::Access, ISS, "other", 300)
            .sign(
                "u",
                Claims {
                    role: "admin".into(),
                },
            )
            .unwrap();
        let forged = token(&other.sk, "admin");
        // expired: signed in the past with the minimum TTL
        let expired = signer(&f.sk, TokenType::Access, ISS, AUD, 1)
            .sign_at(
                std::time::UNIX_EPOCH + Duration::from_secs(1_000_000),
                "u",
                Claims {
                    role: "admin".into(),
                },
            )
            .unwrap();
        let cases: Vec<(Option<String>, &str)> = vec![
            (None, "no header"),
            (Some(format!("Basic {good}")), "basic"),
            (Some("Bearer".into()), "bearer alone"),
            (Some("Bearer ".into()), "bearer empty"),
            (Some(format!("Bearer  {good}")), "two spaces"),
            (Some("Bearer not-a-jwt".into()), "garbage"),
            (Some(format!("Bearer {tampered}")), "tampered"),
            (Some(format!("Bearer {refresh}")), "refresh as access"),
            (Some(format!("Bearer {wrong_iss}")), "iss"),
            (Some(format!("Bearer {wrong_aud}")), "aud"),
            (Some(format!("Bearer {forged}")), "other key"),
            (Some(format!("Bearer {expired}")), "expired"),
        ];
        for (header, label) in cases {
            let (s, h, b) = call(plain_router(&f.auth), header.as_deref()).await;
            assert_generic_401(s, &h, &b, &good);
            assert!(!b.contains(&refresh), "{label}");
        }
    }

    #[tokio::test]
    async fn duplicate_authorization_headers_are_refused() {
        let f = fixture();
        let t = token(&f.sk, "admin");
        let req = axum::http::Request::builder()
            .uri("/me")
            .header(AUTHORIZATION, format!("Bearer {t}"))
            .header(AUTHORIZATION, format!("Bearer {t}"))
            .body(Body::empty())
            .unwrap();
        let (s, h, b) = send(plain_router(&f.auth), req).await;
        assert_generic_401(s, &h, &b, &t);
    }

    #[tokio::test]
    async fn require_claims_distinguishes_401_and_403() {
        let f = fixture();
        let calls = Arc::new(AtomicUsize::new(0));
        let req = |h: Option<String>| {
            let mut b = axum::http::Request::builder().uri("/admin");
            if let Some(h) = h {
                b = b.header(AUTHORIZATION, h);
            }
            b.body(Body::empty()).unwrap()
        };
        let admin = token(&f.sk, "admin");
        let user = token(&f.sk, "user");
        let (s, _, body) = send(
            admin_router(&f.auth, calls.clone()),
            req(Some(format!("Bearer {admin}"))),
        )
        .await;
        assert_eq!((s, body.as_str()), (StatusCode::OK, "user-1"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let (s, h, b) = send(
            admin_router(&f.auth, calls.clone()),
            req(Some(format!("Bearer {user}"))),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        assert!(h.get(WWW_AUTHENTICATE).is_none());
        assert!(!b.contains(&user));

        let (s, h, b) = send(
            admin_router(&f.auth, calls.clone()),
            req(Some("Bearer garbage".into())),
        )
        .await;
        assert_generic_401(s, &h, &b, "garbage");
        let (s, _, _) = send(admin_router(&f.auth, calls.clone()), req(None)).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        // handler ran only for the authorized request
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn authenticated_reuses_the_result_of_require_claims() {
        // Router without AuthConfig::layer: a tower service injects the VerifiedJwt<C> that
        // RequireClaims would have produced; the extractor must reuse it instead of verifying.
        let f = fixture();
        let t = token(&f.sk, "admin");
        let verified: VerifiedJwt<Claims> = f.auth.verifier.verify(&t).unwrap();
        let router = Router::new()
            .route("/me", get(me))
            .layer(axum::middleware::from_fn(
                move |mut req: Request<Body>, next: axum::middleware::Next| {
                    let v = verified.clone();
                    async move {
                        req.extensions_mut().insert(v);
                        next.run(req).await
                    }
                },
            ));
        let (s, _, body) = call(router, None).await;
        assert_eq!((s, body.as_str()), (StatusCode::OK, "user-1:admin"));
    }

    #[tokio::test]
    async fn missing_auth_layer_is_a_500() {
        let t_f = fixture();
        let t = token(&t_f.sk, "admin");
        let router = Router::new().route("/me", get(me));
        let (s, _, b) = call(router, Some(&format!("Bearer {t}"))).await;
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!b.contains(&t));

        let router = Router::new()
            .route("/me", get(|| async { "ok" }))
            .route_layer(RequireClaims::<Claims>::new(|_| true));
        let (s, _, _) = call(router, Some(&format!("Bearer {t}"))).await;
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn auth_config_validates_inputs() {
        let (private, _) = generate_ec_p256_keypair().unwrap();
        let sk = SigningKey::from_pem(private, "k").unwrap();
        let set = || VerifyingKeySet::new(vec![sk.verifying_key()]).unwrap();
        assert_eq!(
            AuthConfig::new(set(), "", AUD).unwrap_err(),
            JwtError::InvalidConfig
        );
        assert_eq!(
            AuthConfig::new(set(), ISS, "").unwrap_err(),
            JwtError::InvalidConfig
        );
        assert_eq!(
            AuthConfig::new(VerifyingKeySet::new(vec![]).unwrap(), ISS, AUD).unwrap_err(),
            JwtError::InvalidConfig
        );
    }

    #[tokio::test]
    async fn leeway_is_applied() {
        let f = fixture();
        let expired = signer(&f.sk, TokenType::Access, ISS, AUD, 10)
            .sign_at(
                std::time::SystemTime::now() - Duration::from_secs(12),
                "u",
                Claims { role: "a".into() },
            )
            .unwrap();
        let hdr = format!("Bearer {expired}");
        let (s, _, _) = call(plain_router(&f.auth), Some(&hdr)).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        let lenient = f.auth.clone().leeway(Duration::from_secs(30));
        let (s, _, _) = call(plain_router(&lenient), Some(&hdr)).await;
        assert_eq!(s, StatusCode::OK);
    }

    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);
    impl Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuf {
        type Writer = LogBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[tokio::test]
    async fn logs_never_contain_the_token() {
        let buf = LogBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::TRACE)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let f = fixture();
        let good = token(&f.sk, "user");
        let calls = Arc::new(AtomicUsize::new(0));
        let _ = call(plain_router(&f.auth), Some(&format!("Bearer {good}x"))).await;
        let _ = call(
            Router::new().route("/me", get(me)),
            Some(&format!("Bearer {good}")),
        )
        .await;
        let req = axum::http::Request::builder()
            .uri("/admin")
            .header(AUTHORIZATION, format!("Bearer {good}"))
            .body(Body::empty())
            .unwrap();
        let _ = send(admin_router(&f.auth, calls), req).await;
        let logs = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("AuthConfig::layer"),
            "500 case should be logged"
        );
        assert!(!logs.contains(&good));
        let payload = good.split('.').nth(1).unwrap();
        assert!(!logs.contains(payload));
    }

    #[tokio::test]
    async fn require_claims_works_with_a_plain_service() {
        // Smoke test of the tower Layer on a bare service.
        let f = fixture();
        let t = token(&f.sk, "admin");
        let inner = service_fn(|_req: Request<Body>| async {
            Ok::<_, Infallible>(Response::new(Body::from("inner")))
        });
        let svc = f
            .auth
            .layer()
            .layer(RequireClaims::<Claims>::new(|c| c.role == "admin").layer(inner));
        let req = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {t}"))
            .body(Body::empty())
            .unwrap();
        let res = svc.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
