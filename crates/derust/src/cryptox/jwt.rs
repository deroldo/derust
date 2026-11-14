//! ES256 JWT signing and verification with `kid`, `token_type`, mandatory `iss`/`aud`.
//!
//! Verification order is fixed: size/shape -> header (own parser) -> `kid` ->
//! signature -> claims. No claim is trusted before the signature is verified.
//! `iss`/`aud` comparisons are exact. There is no default TTL: callers decide.
//! Everything here is instance state; no globals.

use crate::cryptox::error::JwtError;
use crate::cryptox::jose::{jws_sign_es256, jws_verify_es256};
use crate::cryptox::keys::{SigningKey, VerifyingKey, VerifyingKeySet};
use crate::cryptox::util::{b64url_decode, hex_encode, random_bytes, MAX_TOKEN_LEN};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RESERVED_CLAIMS: [&str; 8] = [
    "iss",
    "aud",
    "sub",
    "exp",
    "nbf",
    "iat",
    "jti",
    "token_type",
];
const FORBIDDEN_HEADERS: [&str; 5] = ["crit", "jwk", "jku", "x5u", "x5c"];

/// Kind of token; carried in the `token_type` claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
        }
    }
}

/// Settings of a [`JwtSigner`]. `issuer` and `audience` must be non-empty and `ttl`
/// at least one second; there is intentionally no default TTL.
#[derive(Clone, Debug)]
pub struct JwtSignerSettings {
    pub issuer: String,
    pub audience: String,
    pub ttl: Duration,
}

/// Signs JWTs of one [`TokenType`] with one key.
#[derive(Clone, Debug)]
pub struct JwtSigner {
    key: SigningKey,
    token_type: TokenType,
    issuer: String,
    audience: String,
    ttl_secs: i64,
}

pub(crate) fn check_issuer_audience(issuer: &str, audience: &str) -> Result<(), JwtError> {
    if issuer.is_empty() || audience.is_empty() {
        Err(JwtError::InvalidConfig)
    } else {
        Ok(())
    }
}

pub(crate) fn ttl_to_secs(ttl: Duration) -> Result<i64, JwtError> {
    let secs = i64::try_from(ttl.as_secs()).map_err(|_| JwtError::InvalidConfig)?;
    if secs == 0 {
        Err(JwtError::InvalidConfig)
    } else {
        Ok(secs)
    }
}

fn unix_secs(time: SystemTime) -> Option<i64> {
    let secs = time.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(secs).ok()
}

impl JwtSigner {
    pub fn new(
        key: SigningKey,
        token_type: TokenType,
        settings: JwtSignerSettings,
    ) -> Result<Self, JwtError> {
        check_issuer_audience(&settings.issuer, &settings.audience)?;
        let ttl_secs = ttl_to_secs(settings.ttl)?;
        Ok(Self {
            key,
            token_type,
            issuer: settings.issuer,
            audience: settings.audience,
            ttl_secs,
        })
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type
    }

    /// Signs a token valid from now. `claims` must serialize to a JSON object (or `()`/`None`).
    pub fn sign<C: Serialize>(
        &self,
        subject: impl Into<String>,
        claims: C,
    ) -> Result<String, JwtError> {
        self.sign_at(SystemTime::now(), subject, claims)
    }

    pub fn sign_at<C: Serialize>(
        &self,
        now: SystemTime,
        subject: impl Into<String>,
        claims: C,
    ) -> Result<String, JwtError> {
        let now = unix_secs(now).ok_or(JwtError::SigningFailed)?;
        let exp = now
            .checked_add(self.ttl_secs)
            .ok_or(JwtError::SigningFailed)?;
        let mut payload = match serde_json::to_value(claims).map_err(|_| JwtError::InvalidClaims)? {
            Value::Null => Map::new(),
            Value::Object(map) => map,
            _ => return Err(JwtError::InvalidClaims),
        };
        if RESERVED_CLAIMS.iter().any(|r| payload.contains_key(*r)) {
            return Err(JwtError::ReservedClaim);
        }
        let jti = hex_encode(&random_bytes(16).map_err(|_| JwtError::SigningFailed)?);
        payload.insert("iss".into(), json!(self.issuer));
        payload.insert("aud".into(), json!(self.audience));
        payload.insert("sub".into(), json!(subject.into()));
        payload.insert("iat".into(), json!(now));
        payload.insert("nbf".into(), json!(now));
        payload.insert("exp".into(), json!(exp));
        payload.insert("jti".into(), json!(jti));
        payload.insert("token_type".into(), json!(self.token_type.as_str()));
        let header = json!({"alg": "ES256", "typ": "JWT", "kid": self.key.kid()});
        let header = serde_json::to_vec(&header).map_err(|_| JwtError::SigningFailed)?;
        let payload = serde_json::to_vec(&payload).map_err(|_| JwtError::SigningFailed)?;
        jws_sign_es256(&self.key, &header, &payload).map_err(|_| JwtError::SigningFailed)
    }
}

/// Settings of a [`TokenIssuer`]; no default TTLs.
#[derive(Clone, Debug)]
pub struct TokenIssuerSettings {
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Issues access and refresh tokens signed by two distinct key pairs.
///
/// Plain value: create one per channel (different issuer, audience and TTLs) and they
/// coexist in the same process with no shared state.
#[derive(Clone, Debug)]
pub struct TokenIssuer {
    access: JwtSigner,
    refresh: JwtSigner,
}

impl TokenIssuer {
    /// Fails with `InvalidConfig` if both keys are the same public key (compared by key,
    /// not by `kid`) or if issuer/audience/TTLs are invalid.
    pub fn new(
        access: SigningKey,
        refresh: SigningKey,
        settings: TokenIssuerSettings,
    ) -> Result<Self, JwtError> {
        if access.verifying_key().same_key(&refresh.verifying_key()) {
            return Err(JwtError::InvalidConfig);
        }
        let access = JwtSigner::new(
            access,
            TokenType::Access,
            JwtSignerSettings {
                issuer: settings.issuer.clone(),
                audience: settings.audience.clone(),
                ttl: settings.access_ttl,
            },
        )?;
        let refresh = JwtSigner::new(
            refresh,
            TokenType::Refresh,
            JwtSignerSettings {
                issuer: settings.issuer,
                audience: settings.audience,
                ttl: settings.refresh_ttl,
            },
        )?;
        Ok(Self { access, refresh })
    }

    pub fn access_signer(&self) -> JwtSigner {
        self.access.clone()
    }

    pub fn refresh_signer(&self) -> JwtSigner {
        self.refresh.clone()
    }
}

/// Verifies JWTs of one [`TokenType`] against a key set, issuer and audience.
#[derive(Clone, Debug)]
pub struct JwtVerifier {
    keys: VerifyingKeySet,
    token_type: TokenType,
    issuer: String,
    audience: String,
    leeway_secs: i64,
}

/// A token whose signature and standard claims were verified.
#[derive(Clone, Debug)]
pub struct VerifiedJwt<C> {
    subject: String,
    claims: C,
    expires_at: SystemTime,
    issued_at: SystemTime,
    jti: String,
    kid: String,
    token_type: TokenType,
}

impl<C> VerifiedJwt<C> {
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn claims(&self) -> &C {
        &self.claims
    }
    pub fn into_claims(self) -> C {
        self.claims
    }
    pub fn expires_at(&self) -> SystemTime {
        self.expires_at
    }
    pub fn issued_at(&self) -> SystemTime {
        self.issued_at
    }
    pub fn jti(&self) -> &str {
        &self.jti
    }
    pub fn kid(&self) -> &str {
        &self.kid
    }
    pub fn token_type(&self) -> TokenType {
        self.token_type
    }
}

fn from_unix(secs: i64) -> SystemTime {
    if secs >= 0 {
        UNIX_EPOCH + Duration::from_secs(secs as u64)
    } else {
        UNIX_EPOCH - Duration::from_secs(secs.unsigned_abs())
    }
}

fn decode_object(segment: &str) -> Result<Map<String, Value>, JwtError> {
    let bytes = b64url_decode(segment).ok_or(JwtError::Malformed)?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(JwtError::Malformed),
    }
}

impl JwtVerifier {
    pub fn new(
        keys: VerifyingKeySet,
        token_type: TokenType,
        issuer: impl Into<String>,
        audience: impl Into<String>,
    ) -> Result<Self, JwtError> {
        let (issuer, audience) = (issuer.into(), audience.into());
        check_issuer_audience(&issuer, &audience)?;
        if keys.is_empty() {
            return Err(JwtError::InvalidConfig);
        }
        Ok(Self {
            keys,
            token_type,
            issuer,
            audience,
            leeway_secs: 0,
        })
    }

    /// Clock-skew tolerance for `exp`/`nbf` (default: zero).
    pub fn leeway(mut self, leeway: Duration) -> Self {
        self.leeway_secs = i64::try_from(leeway.as_secs()).unwrap_or(i64::MAX);
        self
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type
    }

    /// `C` receives the complete payload; it must not use `deny_unknown_fields`.
    /// `iat` and `jti` are required (every token made by [`JwtSigner`] carries them).
    pub fn verify<C: DeserializeOwned>(&self, jwt: &str) -> Result<VerifiedJwt<C>, JwtError> {
        self.verify_at(jwt, SystemTime::now())
    }

    pub fn verify_at<C: DeserializeOwned>(
        &self,
        jwt: &str,
        now: SystemTime,
    ) -> Result<VerifiedJwt<C>, JwtError> {
        // 1. size and shape
        if jwt.len() > MAX_TOKEN_LEN {
            return Err(JwtError::Malformed);
        }
        let segments: Vec<&str> = jwt.split('.').collect();
        if segments.len() != 3 || segments.iter().any(|s| b64url_decode(s).is_none()) {
            return Err(JwtError::Malformed);
        }
        // 2. header (own parser) and kid
        let header = decode_object(segments[0])?;
        if header.get("alg").and_then(Value::as_str) != Some("ES256") {
            return Err(JwtError::UnsupportedAlgorithm);
        }
        if FORBIDDEN_HEADERS.iter().any(|h| header.contains_key(*h)) {
            return Err(JwtError::Malformed);
        }
        let key: &VerifyingKey = header
            .get("kid")
            .and_then(Value::as_str)
            .and_then(|kid| self.keys.get(kid))
            .ok_or(JwtError::UnknownKid)?;
        // 3. signature: nothing from the payload is used before this succeeds
        let payload_bytes = jws_verify_es256(key, jwt).map_err(|_| JwtError::BadSignature)?;
        // 4. claims
        let payload = match serde_json::from_slice::<Value>(&payload_bytes) {
            Ok(Value::Object(map)) => map,
            _ => return Err(JwtError::Malformed),
        };
        let now_secs = unix_secs(now).ok_or(JwtError::Malformed)?;
        let exp = payload
            .get("exp")
            .and_then(Value::as_i64)
            .ok_or(JwtError::Malformed)?;
        if now_secs >= exp.saturating_add(self.leeway_secs) {
            return Err(JwtError::Expired);
        }
        if let Some(nbf) = payload.get("nbf") {
            let nbf = nbf.as_i64().ok_or(JwtError::Malformed)?;
            if now_secs.saturating_add(self.leeway_secs) < nbf {
                return Err(JwtError::NotYetValid);
            }
        }
        if payload.get("iss").and_then(Value::as_str) != Some(self.issuer.as_str()) {
            return Err(JwtError::WrongIssuer);
        }
        let aud_ok = match payload.get("aud") {
            Some(Value::String(a)) => *a == self.audience,
            Some(Value::Array(items)) => items
                .iter()
                .any(|v| v.as_str() == Some(self.audience.as_str())),
            _ => false,
        };
        if !aud_ok {
            return Err(JwtError::WrongAudience);
        }
        if payload.get("token_type").and_then(Value::as_str) != Some(self.token_type.as_str()) {
            return Err(JwtError::WrongTokenType);
        }
        let subject = payload
            .get("sub")
            .and_then(Value::as_str)
            .ok_or(JwtError::Malformed)?
            .to_owned();
        let iat = payload
            .get("iat")
            .and_then(Value::as_i64)
            .ok_or(JwtError::Malformed)?;
        let jti = payload
            .get("jti")
            .and_then(Value::as_str)
            .ok_or(JwtError::Malformed)?
            .to_owned();
        // 5. user claims
        let claims: C =
            serde_json::from_value(Value::Object(payload)).map_err(|_| JwtError::Malformed)?;
        Ok(VerifiedJwt {
            subject,
            claims,
            expires_at: from_unix(exp),
            issued_at: from_unix(iat),
            jti,
            kid: key.kid().to_owned(),
            token_type: self.token_type,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::keys::generate_ec_p256_keypair;
    use crate::cryptox::util::b64url_encode;
    use openssl::bn::BigNum;
    use openssl::ec::{EcGroup, EcKey};
    use openssl::ecdsa::EcdsaSig;
    use openssl::hash::MessageDigest;
    use openssl::nid::Nid;
    use openssl::pkey::PKey;
    use openssl::sign::Signer;

    const ISS: &str = "https://id.example";
    const AUD: &str = "bff";

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Claims {
        role: String,
    }

    fn claims() -> Claims {
        Claims {
            role: "admin".into(),
        }
    }

    fn keypair(kid: &str) -> (SigningKey, VerifyingKey) {
        let (private, _) = generate_ec_p256_keypair().unwrap();
        let sk = SigningKey::from_pem(private, kid).unwrap();
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn settings(ttl: u64) -> JwtSignerSettings {
        JwtSignerSettings {
            issuer: ISS.into(),
            audience: AUD.into(),
            ttl: Duration::from_secs(ttl),
        }
    }

    fn setup(tt: TokenType) -> (JwtSigner, JwtVerifier, SigningKey) {
        let (sk, vk) = keypair("k1");
        let signer = JwtSigner::new(sk.clone(), tt, settings(60)).unwrap();
        let verifier =
            JwtVerifier::new(VerifyingKeySet::new(vec![vk]).unwrap(), tt, ISS, AUD).unwrap();
        (signer, verifier, sk)
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn forge(key: &SigningKey, header: Value, payload: Value) -> String {
        jws_sign_es256(
            key,
            &serde_json::to_vec(&header).unwrap(),
            &serde_json::to_vec(&payload).unwrap(),
        )
        .unwrap()
    }

    fn valid_payload() -> Value {
        json!({"iss": ISS, "aud": AUD, "sub": "u", "exp": 2_000_000_100u64, "nbf": 1_999_999_000u64,
               "iat": 1_999_999_000u64, "jti": "j", "token_type": "access", "role": "admin"})
    }

    fn header(kid: &str) -> Value {
        json!({"alg": "ES256", "typ": "JWT", "kid": kid})
    }

    const NOW: u64 = 2_000_000_000;

    #[test]
    fn round_trip() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t1 = signer.sign_at(at(NOW), "user-1", claims()).unwrap();
        let t2 = signer.sign_at(at(NOW), "user-1", claims()).unwrap();
        let v: VerifiedJwt<Claims> = verifier.verify_at(&t1, at(NOW + 1)).unwrap();
        assert_eq!(v.subject(), "user-1");
        assert_eq!(v.claims(), &claims());
        assert_eq!(v.kid(), "k1");
        assert_eq!(v.token_type(), TokenType::Access);
        assert_eq!(v.issued_at(), at(NOW));
        assert_eq!(v.expires_at(), at(NOW + 60));
        let v2: VerifiedJwt<Claims> = verifier.verify_at(&t2, at(NOW + 1)).unwrap();
        assert_ne!(v.jti(), v2.jti());
        assert_eq!(v.jti().len(), 32);
        assert_eq!(v.clone().into_claims(), claims());
    }

    #[test]
    fn unit_claims_are_accepted() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", ()).unwrap();
        assert!(verifier.verify_at::<Value>(&t, at(NOW)).is_ok());
    }

    #[test]
    fn expiry_is_exclusive() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        assert!(verifier.verify_at::<Claims>(&t, at(NOW + 59)).is_ok());
        assert_eq!(
            verifier.verify_at::<Claims>(&t, at(NOW + 60)).unwrap_err(),
            JwtError::Expired
        );
    }

    #[test]
    fn leeway_extends_expiry_and_nbf() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let verifier = verifier.leeway(Duration::from_secs(5));
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        assert!(verifier.verify_at::<Claims>(&t, at(NOW + 64)).is_ok());
        assert_eq!(
            verifier.verify_at::<Claims>(&t, at(NOW + 66)).unwrap_err(),
            JwtError::Expired
        );
        assert!(verifier.verify_at::<Claims>(&t, at(NOW - 3)).is_ok());
        assert_eq!(
            verifier.verify_at::<Claims>(&t, at(NOW - 6)).unwrap_err(),
            JwtError::NotYetValid
        );
    }

    #[test]
    fn nbf_in_future_without_leeway() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        assert_eq!(
            verifier.verify_at::<Claims>(&t, at(NOW - 1)).unwrap_err(),
            JwtError::NotYetValid
        );
    }

    #[test]
    fn wrong_or_missing_issuer_and_audience() {
        let (_, verifier, sk) = setup(TokenType::Access);
        let mut p = valid_payload();
        p["iss"] = json!("other");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongIssuer
        );
        let mut p = valid_payload();
        p.as_object_mut().unwrap().remove("iss");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongIssuer
        );
        let mut p = valid_payload();
        p["aud"] = json!("other");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongAudience
        );
        let mut p = valid_payload();
        p.as_object_mut().unwrap().remove("aud");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongAudience
        );
    }

    #[test]
    fn audience_array() {
        let (_, verifier, sk) = setup(TokenType::Access);
        let mut p = valid_payload();
        p["aud"] = json!(["x", AUD]);
        assert!(verifier
            .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
            .is_ok());
        let mut p = valid_payload();
        p["aud"] = json!(["x", "y"]);
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongAudience
        );
    }

    #[test]
    fn tampered_segments_are_never_accepted() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        let parts: Vec<&str> = t.split('.').collect();
        for idx in 0..3 {
            let mut bytes = b64url_decode(parts[idx]).unwrap();
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            let mut p: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
            p[idx] = b64url_encode(&bytes);
            let tampered = p.join(".");
            let err = verifier
                .verify_at::<Claims>(&tampered, at(NOW))
                .unwrap_err();
            assert!(
                matches!(err, JwtError::BadSignature | JwtError::Malformed),
                "segment {idx}: {err:?}"
            );
        }
    }

    #[test]
    fn signature_is_checked_before_claims() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        let parts: Vec<&str> = t.split('.').collect();
        // payload claims that would be rejected (wrong issuer) if read before the signature
        let mut evil = valid_payload();
        evil["iss"] = json!("evil");
        let forged = format!(
            "{}.{}.{}",
            parts[0],
            b64url_encode(&serde_json::to_vec(&evil).unwrap()),
            parts[2]
        );
        assert_eq!(
            verifier.verify_at::<Value>(&forged, at(NOW)).unwrap_err(),
            JwtError::BadSignature
        );
    }

    #[test]
    fn hs256_rs256_none_and_lowercase_are_unsupported() {
        let (signer, verifier, sk) = setup(TokenType::Access);
        let legit = signer.sign_at(at(NOW), "u", claims()).unwrap();
        let payload_seg = legit.split('.').nth(1).unwrap().to_owned();
        // HS256 using the public key PEM as HMAC secret (classic confusion attack)
        let public_pem = sk.verifying_key().pkey().public_key_to_pem().unwrap();
        let hs_header = b64url_encode(br#"{"alg":"HS256","typ":"JWT","kid":"k1"}"#);
        let input = format!("{hs_header}.{payload_seg}");
        let mac_key = PKey::hmac(&public_pem).unwrap();
        let mut s = Signer::new(MessageDigest::sha256(), &mac_key).unwrap();
        s.update(input.as_bytes()).unwrap();
        let hs = format!("{input}.{}", b64url_encode(&s.sign_to_vec().unwrap()));
        assert_eq!(
            verifier.verify_at::<Value>(&hs, at(NOW)).unwrap_err(),
            JwtError::UnsupportedAlgorithm
        );
        for alg in [
            r#"{"alg":"RS256","kid":"k1"}"#,
            r#"{"alg":"none","kid":"k1"}"#,
            r#"{"alg":"es256","kid":"k1"}"#,
            r#"{"kid":"k1"}"#,
        ] {
            let h = b64url_encode(alg.as_bytes());
            for sig in ["", "AAAA"] {
                let t = format!("{h}.{payload_seg}.{sig}");
                assert_eq!(
                    verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
                    JwtError::UnsupportedAlgorithm,
                    "{alg} / sig {sig:?}"
                );
            }
        }
    }

    #[test]
    fn unknown_and_missing_kid() {
        let (_, verifier, sk) = setup(TokenType::Access);
        let t = forge(&sk, header("nope"), valid_payload());
        assert_eq!(
            verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
            JwtError::UnknownKid
        );
        let t = forge(&sk, json!({"alg": "ES256"}), valid_payload());
        assert_eq!(
            verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
            JwtError::UnknownKid
        );
        let t = forge(&sk, json!({"alg": "ES256", "kid": 7}), valid_payload());
        assert_eq!(
            verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
            JwtError::UnknownKid
        );
    }

    #[test]
    fn forbidden_headers_are_rejected() {
        let (_, verifier, sk) = setup(TokenType::Access);
        for extra in ["crit", "jwk", "jku", "x5u", "x5c"] {
            let mut h = header("k1");
            h[extra] = json!(["x"]);
            let t = forge(&sk, h, valid_payload());
            assert_eq!(
                verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
                JwtError::Malformed,
                "{extra}"
            );
        }
    }

    #[test]
    fn signature_from_another_key_with_known_kid() {
        let (_, verifier, _) = setup(TokenType::Access);
        let (other, _) = keypair("k1");
        let t = forge(&other, header("k1"), valid_payload());
        assert_eq!(
            verifier.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
            JwtError::BadSignature
        );
    }

    #[test]
    fn token_types_are_not_interchangeable() {
        let (sk, vk) = keypair("k1");
        let set = VerifyingKeySet::new(vec![vk]).unwrap();
        let access = JwtSigner::new(sk.clone(), TokenType::Access, settings(60)).unwrap();
        let refresh = JwtSigner::new(sk, TokenType::Refresh, settings(60)).unwrap();
        let v_access = JwtVerifier::new(set.clone(), TokenType::Access, ISS, AUD).unwrap();
        let v_refresh = JwtVerifier::new(set, TokenType::Refresh, ISS, AUD).unwrap();
        let ta = access.sign_at(at(NOW), "u", ()).unwrap();
        let tr = refresh.sign_at(at(NOW), "u", ()).unwrap();
        assert!(v_access.verify_at::<Value>(&ta, at(NOW)).is_ok());
        assert!(v_refresh.verify_at::<Value>(&tr, at(NOW)).is_ok());
        assert_eq!(
            v_access.verify_at::<Value>(&tr, at(NOW)).unwrap_err(),
            JwtError::WrongTokenType
        );
        assert_eq!(
            v_refresh.verify_at::<Value>(&ta, at(NOW)).unwrap_err(),
            JwtError::WrongTokenType
        );
    }

    #[test]
    fn separate_key_sets_give_unknown_kid() {
        let (sk_a, vk_a) = keypair("access-1");
        let (_, vk_r) = keypair("refresh-1");
        let signer = JwtSigner::new(sk_a, TokenType::Access, settings(60)).unwrap();
        let v_refresh = JwtVerifier::new(
            VerifyingKeySet::new(vec![vk_r]).unwrap(),
            TokenType::Refresh,
            ISS,
            AUD,
        )
        .unwrap();
        let t = signer.sign_at(at(NOW), "u", ()).unwrap();
        assert_eq!(
            v_refresh.verify_at::<Value>(&t, at(NOW)).unwrap_err(),
            JwtError::UnknownKid
        );
        let _ = vk_a;
    }

    #[test]
    fn missing_standard_claims() {
        let (_, verifier, sk) = setup(TokenType::Access);
        let mut p = valid_payload();
        p.as_object_mut().unwrap().remove("token_type");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::WrongTokenType
        );
        for claim in ["exp", "sub", "iat", "jti"] {
            let mut p = valid_payload();
            p.as_object_mut().unwrap().remove(claim);
            assert_eq!(
                verifier
                    .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                    .unwrap_err(),
                JwtError::Malformed,
                "{claim}"
            );
        }
        let mut p = valid_payload();
        p["exp"] = json!("tomorrow");
        assert_eq!(
            verifier
                .verify_at::<Value>(&forge(&sk, header("k1"), p), at(NOW))
                .unwrap_err(),
            JwtError::Malformed
        );
    }

    #[test]
    fn custom_claims_type_mismatch_is_malformed() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", ()).unwrap();
        assert_eq!(
            verifier.verify_at::<Claims>(&t, at(NOW)).unwrap_err(),
            JwtError::Malformed
        );
    }

    #[test]
    fn reserved_claims_are_refused() {
        let (signer, _, _) = setup(TokenType::Access);
        for name in RESERVED_CLAIMS {
            let mut m = Map::new();
            m.insert(name.into(), json!("x"));
            assert_eq!(
                signer.sign("u", m).unwrap_err(),
                JwtError::ReservedClaim,
                "{name}"
            );
        }
    }

    #[test]
    fn non_object_claims_are_invalid() {
        let (signer, _, _) = setup(TokenType::Access);
        assert_eq!(signer.sign("u", 5).unwrap_err(), JwtError::InvalidClaims);
        assert_eq!(
            signer.sign("u", vec![1, 2]).unwrap_err(),
            JwtError::InvalidClaims
        );
        assert_eq!(
            signer.sign("u", "text").unwrap_err(),
            JwtError::InvalidClaims
        );
    }

    #[test]
    fn constructors_validate_configuration() {
        let (sk, vk) = keypair("k1");
        let set = VerifyingKeySet::new(vec![vk]).unwrap();
        let bad = |iss: &str, aud: &str, ttl: u64| {
            JwtSigner::new(
                sk.clone(),
                TokenType::Access,
                JwtSignerSettings {
                    issuer: iss.into(),
                    audience: aud.into(),
                    ttl: Duration::from_secs(ttl),
                },
            )
            .unwrap_err()
        };
        assert_eq!(bad("", AUD, 60), JwtError::InvalidConfig);
        assert_eq!(bad(ISS, "", 60), JwtError::InvalidConfig);
        assert_eq!(bad(ISS, AUD, 0), JwtError::InvalidConfig);
        assert_eq!(
            JwtVerifier::new(set.clone(), TokenType::Access, "", AUD).unwrap_err(),
            JwtError::InvalidConfig
        );
        assert_eq!(
            JwtVerifier::new(set, TokenType::Access, ISS, "").unwrap_err(),
            JwtError::InvalidConfig
        );
        assert_eq!(
            JwtVerifier::new(
                VerifyingKeySet::new(vec![]).unwrap(),
                TokenType::Access,
                ISS,
                AUD
            )
            .unwrap_err(),
            JwtError::InvalidConfig
        );
    }

    #[test]
    fn size_limit_and_garbage_are_malformed() {
        let (_, verifier, _) = setup(TokenType::Access);
        let huge = format!("{}.{}.{}", "a".repeat(MAX_TOKEN_LEN), "b", "c");
        for t in [huge.as_str(), "", "a.b", "a.b.c.d", "!!!.@@@.###"] {
            assert_eq!(
                verifier.verify_at::<Value>(t, at(NOW)).unwrap_err(),
                JwtError::Malformed,
                "{t}"
            );
        }
    }

    #[test]
    fn debug_and_display_do_not_expose_the_token() {
        let (signer, verifier, _) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        let v: VerifiedJwt<Claims> = verifier.verify_at(&t, at(NOW)).unwrap();
        assert!(!format!("{v:?}").contains(&t));
        let tampered = format!("{t}x");
        let err = verifier
            .verify_at::<Claims>(&tampered, at(NOW))
            .unwrap_err();
        assert!(!format!("{err:?} {err}").contains(&tampered));
        assert!(!format!("{signer:?}").contains("PRIVATE"));
    }

    fn issuer_settings(iss: &str, aud: &str, a: u64, r: u64) -> TokenIssuerSettings {
        TokenIssuerSettings {
            issuer: iss.into(),
            audience: aud.into(),
            access_ttl: Duration::from_secs(a),
            refresh_ttl: Duration::from_secs(r),
        }
    }

    fn verifier_for(vk: VerifyingKey, tt: TokenType, iss: &str, aud: &str) -> JwtVerifier {
        JwtVerifier::new(VerifyingKeySet::new(vec![vk]).unwrap(), tt, iss, aud).unwrap()
    }

    #[test]
    fn issuer_with_distinct_pairs_issues_verifiable_tokens() {
        let (a, avk) = keypair("a1");
        let (r, rvk) = keypair("r1");
        let issuer = TokenIssuer::new(a, r, issuer_settings(ISS, AUD, 900, 86_400)).unwrap();
        let at_ = issuer.access_signer().sign_at(at(NOW), "u", ()).unwrap();
        let rt = issuer.refresh_signer().sign_at(at(NOW), "u", ()).unwrap();
        let va = verifier_for(avk.clone(), TokenType::Access, ISS, AUD);
        let vr = verifier_for(rvk.clone(), TokenType::Refresh, ISS, AUD);
        let acc: VerifiedJwt<Value> = va.verify_at(&at_, at(NOW)).unwrap();
        let ref_: VerifiedJwt<Value> = vr.verify_at(&rt, at(NOW)).unwrap();
        assert_eq!(
            acc.expires_at().duration_since(acc.issued_at()).unwrap(),
            Duration::from_secs(900)
        );
        assert_eq!(
            ref_.expires_at().duration_since(ref_.issued_at()).unwrap(),
            Duration::from_secs(86_400)
        );
        // cross use fails: wrong token type with a shared set, unknown kid with separate sets
        let both = VerifyingKeySet::new(vec![avk, rvk]).unwrap();
        let va_both = JwtVerifier::new(both.clone(), TokenType::Access, ISS, AUD).unwrap();
        let vr_both = JwtVerifier::new(both, TokenType::Refresh, ISS, AUD).unwrap();
        assert_eq!(
            va_both.verify_at::<Value>(&rt, at(NOW)).unwrap_err(),
            JwtError::WrongTokenType
        );
        assert_eq!(
            vr_both.verify_at::<Value>(&at_, at(NOW)).unwrap_err(),
            JwtError::WrongTokenType
        );
        assert_eq!(
            va.verify_at::<Value>(&rt, at(NOW)).unwrap_err(),
            JwtError::UnknownKid
        );
    }

    #[test]
    fn issuer_rejects_the_same_key_even_with_different_kids() {
        let (private, _) = generate_ec_p256_keypair().unwrap();
        let a = SigningKey::from_pem(&private, "a").unwrap();
        let r = SigningKey::from_pem(&private, "r").unwrap();
        assert_eq!(
            TokenIssuer::new(a, r, issuer_settings(ISS, AUD, 60, 120)).unwrap_err(),
            JwtError::InvalidConfig
        );
        let a = SigningKey::from_pem(&private, "same").unwrap();
        let r = SigningKey::from_pem(&private, "same").unwrap();
        assert_eq!(
            TokenIssuer::new(a, r, issuer_settings(ISS, AUD, 60, 120)).unwrap_err(),
            JwtError::InvalidConfig
        );
    }

    #[test]
    fn issuer_accepts_different_keys_with_the_same_kid() {
        let (a, _) = keypair("same");
        let (r, _) = keypair("same");
        assert!(TokenIssuer::new(a, r, issuer_settings(ISS, AUD, 60, 120)).is_ok());
    }

    #[test]
    fn issuer_validates_settings() {
        let mk = |s: TokenIssuerSettings| {
            let (a, _) = keypair("a");
            let (r, _) = keypair("r");
            TokenIssuer::new(a, r, s).unwrap_err()
        };
        assert_eq!(
            mk(issuer_settings("", AUD, 60, 60)),
            JwtError::InvalidConfig
        );
        assert_eq!(
            mk(issuer_settings(ISS, "", 60, 60)),
            JwtError::InvalidConfig
        );
        assert_eq!(
            mk(issuer_settings(ISS, AUD, 0, 60)),
            JwtError::InvalidConfig
        );
        assert_eq!(
            mk(issuer_settings(ISS, AUD, 60, 0)),
            JwtError::InvalidConfig
        );
    }

    #[test]
    fn two_issuers_coexist_without_shared_state() {
        // One issuer per channel: distinct TTLs, issuer and audience, same process.
        let (wa, wavk) = keypair("web-a");
        let (wr, _) = keypair("web-r");
        let (ma, mavk) = keypair("mob-a");
        let (mr, _) = keypair("mob-r");
        let web =
            TokenIssuer::new(wa, wr, issuer_settings("id-web", "bff-web", 300, 3_600)).unwrap();
        let mobile = TokenIssuer::new(
            ma,
            mr,
            issuer_settings("id-mobile", "bff-mobile", 1_800, 2_592_000),
        )
        .unwrap();

        let web_token = web.access_signer().sign_at(at(NOW), "u", ()).unwrap();
        let mobile_token = mobile.access_signer().sign_at(at(NOW), "u", ()).unwrap();
        let web_v = verifier_for(wavk, TokenType::Access, "id-web", "bff-web");
        let mobile_v = verifier_for(mavk, TokenType::Access, "id-mobile", "bff-mobile");

        let w: VerifiedJwt<Value> = web_v.verify_at(&web_token, at(NOW)).unwrap();
        let m: VerifiedJwt<Value> = mobile_v.verify_at(&mobile_token, at(NOW)).unwrap();
        assert_eq!(w.expires_at(), at(NOW + 300));
        assert_eq!(m.expires_at(), at(NOW + 1_800));
        assert_eq!(w.kid(), "web-a");
        assert_eq!(m.kid(), "mob-a");
        // each channel's token is rejected by the other channel's verifier
        assert!(web_v.verify_at::<Value>(&mobile_token, at(NOW)).is_err());
        assert!(mobile_v.verify_at::<Value>(&web_token, at(NOW)).is_err());
        // signing with one does not alter the other (interleaved, repeated)
        let again = web.access_signer().sign_at(at(NOW + 10), "u", ()).unwrap();
        let w2: VerifiedJwt<Value> = web_v.verify_at(&again, at(NOW + 10)).unwrap();
        assert_eq!(w2.expires_at(), at(NOW + 310));
        let m2: VerifiedJwt<Value> = mobile_v
            .verify_at(
                &mobile
                    .access_signer()
                    .sign_at(at(NOW + 10), "u", ())
                    .unwrap(),
                at(NOW + 10),
            )
            .unwrap();
        assert_eq!(m2.expires_at(), at(NOW + 1_810));
    }

    // RFC 7515 Appendix A.3 (ES256 example).
    const RFC7515_A3: &str = "eyJhbGciOiJFUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

    fn rfc_key() -> VerifyingKey {
        let x = BigNum::from_slice(
            &b64url_decode("f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU").unwrap(),
        )
        .unwrap();
        let y = BigNum::from_slice(
            &b64url_decode("x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0").unwrap(),
        )
        .unwrap();
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ec = EcKey::from_public_key_affine_coordinates(&group, &x, &y).unwrap();
        let pem = PKey::from_ec_key(ec).unwrap().public_key_to_pem().unwrap();
        VerifyingKey::from_pem(pem, "rfc").unwrap()
    }

    #[test]
    fn rfc7515_a3_vector_verifies() {
        let payload = jws_verify_es256(&rfc_key(), RFC7515_A3).unwrap();
        assert!(String::from_utf8(payload)
            .unwrap()
            .contains("\"iss\":\"joe\""));
        // tampered vector must fail
        let bad = RFC7515_A3.replacen("eyJpc3Mi", "eyJpc3Ni", 1);
        assert!(jws_verify_es256(&rfc_key(), &bad).is_err());
    }

    #[test]
    fn signature_verifies_with_plain_openssl() {
        let (signer, _, sk) = setup(TokenType::Access);
        let t = signer.sign_at(at(NOW), "u", claims()).unwrap();
        let (input, sig) = t.rsplit_once('.').unwrap();
        let raw = b64url_decode(sig).unwrap();
        assert_eq!(raw.len(), 64);
        let r = BigNum::from_slice(&raw[..32]).unwrap();
        let s = BigNum::from_slice(&raw[32..]).unwrap();
        let sig = EcdsaSig::from_private_components(r, s).unwrap();
        let digest = openssl::hash::hash(MessageDigest::sha256(), input.as_bytes()).unwrap();
        let ec = sk.verifying_key().pkey().ec_key().unwrap();
        assert!(sig.verify(&digest, &ec).unwrap());
    }
}
