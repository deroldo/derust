# cryptox

Feature `cryptox` (module `derust::cryptox`): JWT/JOSE, encryption and hashing helpers for services that
issue or validate tokens. It is independent of the existing `httpx::protect_endpoints_core` extractor
(HS/RS keys via `jsonwebtoken`); nothing from `cryptox` is re-exported there or at the crate root.

```toml
derust = { version = "0.7", features = ["cryptox"] }
```

`cryptox` alone does not pull `http_server`. The Axum integration (`derust::cryptox::http`) only exists
when `http_server` is enabled (it is a default feature). `SecretsManagerKeySource` needs `aws`.

## What is in it

| Item | Purpose |
|---|---|
| `SigningKey`, `VerifyingKey`, `VerifyingKeySet` | EC P-256 keys with a `kid`; JWKS via `to_jwks()` |
| `JwtSigner`, `JwtVerifier`, `TokenIssuer` | ES256 JWT with `kid` and `token_type` (access / refresh) |
| `JweSealer`, `JweOpener`, `JweOpenerSet`, `Sealer` | compact JWE, `ECDH-ES` + `A256GCM`, optional `kid` |
| `PasswordHasher`, `Argon2Params` | Argon2id (PHC string), `needs_rehash`, `verify_or_dummy` |
| `Pepper`, `CodeHasher`, `generate_numeric_code` | short codes (OTP): HMAC-SHA256 with a secret pepper |
| `KeySource`, `*Config`, `KeySourceResolver` | serde config; env var, file or AWS Secrets Manager |
| `http::{AuthConfig, Authenticated, RequireClaims}` | Axum integration (`http_server`) |

Nothing is global or static: every type is a plain value, so several configurations (for instance one
`TokenIssuer` per channel, each with its own issuer, audience and TTLs) coexist in the same process.

## Keys and PEM formats

All keys are EC P-256. Private keys are PKCS#8 (`-----BEGIN PRIVATE KEY-----`), public keys are SPKI
(`-----BEGIN PUBLIC KEY-----`). RSA, other curves, SEC1 (`EC PRIVATE KEY`) and certificates are rejected
when the key is created (`KeyError`). `generate_ec_p256_keypair()` returns `(private, public)` PEMs.
`VerifyingKeySet::new` fails with `DuplicateKid` and returns `Result`; `PasswordHasher::new` also
returns `Result`.

## JWT

- Header `{"alg":"ES256","typ":"JWT","kid":...}`; claims `iss`, `aud`, `sub`, `iat`, `nbf`, `exp`, `jti`
  and `token_type` (`"access"` / `"refresh"`). User claims cannot use those names (`ReservedClaim`).
- **`iss` and `aud` are mandatory** constructor parameters (empty -> `JwtError::InvalidConfig`) in
  `JwtSigner`/`JwtSignerSettings`, `TokenIssuerSettings`, `JwtVerifier::new` and `AuthConfig::new`.
  Comparison is exact.
- **There is no default TTL**: `JwtSignerSettings::ttl` and `TokenIssuerSettings::{access_ttl, refresh_ttl}`
  are required (minimum one second). `leeway` defaults to zero.
- Verification order is fixed: size/shape, header (own parser: `alg` must be exactly `ES256`; `crit`,
  `jwk`, `jku`, `x5u`, `x5c` are rejected), `kid`, signature, then claims. No claim is trusted before the
  signature is verified. `iat` and `jti` must be present. Tokens above 16 KiB are `Malformed`.
- Rotation: publish several keys in the verifier's `VerifyingKeySet` (selected by `kid`); sign with the
  new key. `TokenIssuer` refuses the same public key for access and refresh (`InvalidConfig`), and the
  `token_type` claim stops one kind from being used as the other.
- `TokenIssuer` derives the verification material from its signing keys: `access_verifying_keys()`,
  `access_verifier()`, `refresh_verifier()` (same issuer/audience, `TokenType::Refresh`) and `jwks()` (access
  key only: never the refresh key nor private material), so an issuer can validate its own refresh tokens.
- `VerifyingKeySet::from_jwks` / `from_jwks_value` rebuild a set from a JWKS (see config below).
- The claims type `C` of `verify` receives the whole payload; do not use `deny_unknown_fields`.

## JWE

`JweSealer::new(public_pem)` / `JweOpener::new(private_pem)`. `alg` is `ECDH-ES` (direct, empty
encrypted key), `enc` is `A256GCM`, and every `seal` generates a fresh ephemeral key. `open` rejects other
algorithms, `zip`, `crit`, an `epk` that is not P-256 and inputs above 64 KiB; any cryptographic failure
is a single `DecryptFailed`. The `kid` is optional: `JweSealer::with_kid(..)` puts it in the protected header (<= 128 bytes) and
`JweOpenerSet` (one `JweOpener` per `kid`) picks the recipient key by it during rotation; a plain `JweOpener`
still opens JWEs with or without `kid`. In the set, a missing or unknown `kid` and any cryptographic failure are
the same single `DecryptFailed` (the `kid` is only read to choose the key, before decrypting). Inject `&dyn Sealer` in services to fake it in tests.
Interoperability with Node `jose` was verified in both directions
(`crates/derust/tests/cryptox/README.md`).

## Passwords versus short codes

Passwords are low entropy and need a slow, salted hash: Argon2id (default 19 MiB, t=2, p=1, the OWASP
minimum; `hash`/`verify` are CPU-bound, run them in `tokio::task::spawn_blocking`). Short numeric codes
have so little entropy that no slow hash protects them offline; they are stored as
`HMAC-SHA256(pepper, code)` where the pepper (`Pepper::from_bytes`, validated: >= 32 bytes, zeroized, redacted `Debug`) is random and kept **outside the database**.
Attempt limiting and a TTL are the service's job.

`PasswordHasher::verify_or_dummy(password, stored: Option<&str>)` equalizes login timing: for an unknown user
(`None`) it verifies against a dummy hash built with the same parameters (computed once in `new`, kept in the
instance, no global state) and returns `false`. A malformed `stored` still performs the dummy verification before
returning `Err`; treat that `Err` as an internal data problem and answer the client as for a wrong password.
Because of the dummy, `PasswordHasher::new` costs one extra Argon2 hash. `generate_numeric_code` has no modulo bias.

## Config and key sources

`KeySourceConfig` (`source = "env" | "file" | "secrets_manager"`), `SigningKeyConfig`,
`VerifyingKeyConfig`, `JwtVerifierConfig::build(&resolver, TokenType)` and `TokenIssuerConfig::build(&resolver)`
are `Deserialize` and hold no PEM, only where to read it. Env sources convert a literal `\n` to a line
break. **Lists** (`JwtVerifierConfig::keys`) cannot be read from environment variables by index (the `config`
crate used by `envx` does not support `KEYS__0__KID`). For several keys (rotation) use
`JwtVerifierConfig::jwks` instead: ONE JWKS text read through the same `KeySourceConfig` (env var, file or
Secrets Manager), e.g. `APP__JWKS__SOURCE=env` + `APP__JWKS__VAR=MY_JWKS`. It is parsed by
`VerifyingKeySet::from_jwks` (or `from_jwks_value`), which is strict: only EC P-256 public keys, `kid`
required, `use`/`alg` (when present) must be `sig`/`ES256`, private parameters (`d`), other key types or
curves, duplicate `kid`, empty sets and documents over 64 KiB are errors, never skipped; it round-trips
with `to_jwks()`. Set `keys` or `jwks`, not both. Fetching a JWKS by URL is not implemented.
Single-valued settings (`TokenIssuerConfig`) work with `APP__...` variables.
The real Secrets Manager call is not covered by unit tests; validate it manually with localstack.
`aws` currently requires `http_server`/`env` because of `awsx`.

## Axum (`http_server`)

`AuthConfig::layer()` puts the verifier in the request extensions (the orphan rule prevents `FromRef`
on `AppContext`); `Authenticated<C>` extracts the verified token and `RequireClaims<C>` checks a predicate.
Apply `.layer(auth.layer())` **after** (outside of) `route_layer`. Responses: `401` with
`WWW-Authenticate: Bearer` and an empty body (reason only in `DerustTags`, never the token), `403` when the
predicate is false, `500` when `AuthConfig::layer()` is missing. `RequireClaims` stores its result so
`Authenticated` does not verify twice.

## Examples

Issuer (identity-service), including one issuer per channel, JWKS, JWE, passwords and codes:

```rust
use derust::cryptox::{
    generate_ec_p256_keypair, generate_numeric_code, Argon2Params, CodeHasher, JweOpener,
    JweOpenerSet, JweSealer, PasswordHasher, Pepper, Sealer, SigningKey, TokenIssuer,
    TokenIssuerSettings, VerifyingKeySet,
};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct MyClaims {
    role: String,
}

fn issuer_for(channel: &str, ttl: (u64, u64)) -> Result<(TokenIssuer, VerifyingKeySet), Box<dyn std::error::Error>> {
    // In a service the PEMs come from FileKeySource / Secrets Manager (see `KeySourceConfig`).
    let (access_pem, _) = generate_ec_p256_keypair()?;
    let (refresh_pem, _) = generate_ec_p256_keypair()?;
    let access = SigningKey::from_pem(access_pem, format!("{channel}-access-1"))?;
    let refresh = SigningKey::from_pem(refresh_pem, format!("{channel}-refresh-1"))?;
    let jwks_keys = VerifyingKeySet::new(vec![access.verifying_key()])?;
    let issuer = TokenIssuer::new(
        access,
        refresh,
        TokenIssuerSettings {
            issuer: format!("https://id.example/{channel}"),
            audience: format!("bff-{channel}"),
            access_ttl: Duration::from_secs(ttl.0),
            refresh_ttl: Duration::from_secs(ttl.1),
        },
    )?;
    Ok((issuer, jwks_keys))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (web, web_keys) = issuer_for("web", (900, 30 * 86_400))?;
    let (mobile, _) = issuer_for("mobile", (1_800, 90 * 86_400))?;

    let access_token = web.access_signer().sign("user-1", MyClaims { role: "admin".into() })?;
    let mobile_refresh = mobile.refresh_signer().sign("user-1", ())?;
    assert_ne!(access_token, mobile_refresh);
    let jwks = web.jwks(); // access key only; publish at /.well-known/jwks.json
    assert!(jwks["keys"][0].get("d").is_none());
    assert_eq!(jwks, web_keys.to_jwks());
    // A consumer (BFF) rebuilds the key set from the JWKS text (rotation without env lists).
    let remote = VerifyingKeySet::from_jwks(&jwks.to_string())?;
    assert_eq!(remote.len(), 1);

    // The issuer validates its own tokens: derived verifiers share issuer/audience.
    let refresh_token = web.refresh_signer().sign("user-1", ())?;
    let verified = web.refresh_verifier().verify::<serde_json::Value>(&refresh_token)?;
    assert_eq!(verified.subject(), "user-1");
    assert!(web.access_verifier().verify::<serde_json::Value>(&refresh_token).is_err());

    // JWE: seal to the recipient's public key, only the private key opens it.
    let (jwe_private, jwe_public) = generate_ec_p256_keypair()?;
    let sealed = JweSealer::new(&jwe_public)?.seal(b"sensitive payload")?;
    assert_eq!(JweOpener::new(&jwe_private)?.open(&sealed)?, b"sensitive payload");
    // JWE `kid` is optional; with it, the recipient can rotate its key (pick the key by `kid`).
    let sealed_kid = JweSealer::new(&jwe_public)?.with_kid("recipient-2026")?.seal(b"rotating")?;
    let openers = JweOpenerSet::new(vec![("recipient-2026".into(), JweOpener::new(&jwe_private)?)])?;
    assert_eq!(openers.open(&sealed_kid)?, b"rotating");

    // Passwords: Argon2id (CPU-bound: use spawn_blocking in async code).
    let hasher = PasswordHasher::new(Argon2Params::default())?;
    let phc = hasher.hash("correct horse battery staple")?;
    assert!(hasher.verify("correct horse battery staple", &phc)?);
    assert!(!hasher.needs_rehash(&phc));
    // Login: unknown user (`None`) costs the same as a wrong password and returns `false`.
    assert!(hasher.verify_or_dummy("correct horse battery staple", Some(&phc))?);
    assert!(!hasher.verify_or_dummy("anything", None)?);

    // Short codes (OTP): random digits, stored as HMAC-SHA256 with a secret pepper.
    let pepper = Pepper::from_bytes(b"0123456789abcdef0123456789abcdef".to_vec())?; // load >= 32 random bytes from a secret store
    let codes = CodeHasher::new(pepper);
    let code = generate_numeric_code(6);
    let stored = codes.hash(&code);
    assert!(codes.verify(&code, &stored));
    Ok(())
}
```

Validator (BFF) with `JwtVerifier` and `Authenticated`:

```rust
use axum::{routing::get, Router};
use derust::cryptox::http::{AuthConfig, Authenticated, RequireClaims};
use derust::cryptox::{
    generate_ec_p256_keypair, JwtSigner, JwtSignerSettings, JwtVerifier, SigningKey, TokenType,
    VerifiedJwt, VerifyingKeySet,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    role: String,
}

async fn me(Authenticated(jwt): Authenticated<Claims>) -> String {
    jwt.subject().to_owned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In the BFF the public keys come from a file / Secrets Manager (`JwtVerifierConfig`).
    let (private_pem, _) = generate_ec_p256_keypair()?;
    let signing = SigningKey::from_pem(private_pem, "k1")?;
    let set = VerifyingKeySet::new(vec![signing.verifying_key()])?;

    // Direct verification (iss and aud are mandatory):
    let signer = JwtSigner::new(
        signing,
        TokenType::Access,
        JwtSignerSettings {
            issuer: "https://id.example".into(),
            audience: "bff".into(),
            ttl: Duration::from_secs(900),
        },
    )?;
    let token = signer.sign("user-1", Claims { role: "admin".into() })?;
    let verifier = JwtVerifier::new(set.clone(), TokenType::Access, "https://id.example", "bff")?;
    let jwt: VerifiedJwt<Claims> = verifier.verify(&token)?;
    assert_eq!(jwt.claims().role, "admin");

    // Axum: `.layer(auth.layer())` goes after (outside of) `route_layer`.
    let auth = AuthConfig::new(set, "https://id.example", "bff")?;
    let _router: Router = Router::new()
        .route("/me", get(me))
        .route_layer(RequireClaims::<Claims>::new(|c| c.role == "admin"))
        .layer(auth.layer());
    Ok(())
}
```

Both are doctests (`cargo test --doc --package derust --features cryptox`).

## Limits and risks

- Keys live inside OpenSSL objects and are not zeroized; PEM text, the pepper and the JWE private PEM are
  held in `Zeroizing` buffers.
- `josekit` is stagnant and pinned to `=0.10.3`; it is isolated in `cryptox/jose.rs`, and the
  header and claims are validated by `cryptox` itself, never by `josekit` validators.
- Secrets (tokens, PEM, codes, peppers, hashes) never appear in `Display`/`Debug` of errors or in logs.
