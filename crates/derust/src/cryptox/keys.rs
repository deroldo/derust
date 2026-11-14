//! EC P-256 keys in PEM (PKCS#8 private, SPKI public), key sets and JWKS.

use crate::cryptox::error::KeyError;
use crate::cryptox::util::{b64url_decode, b64url_encode, MAX_JWKS_LEN};
use openssl::bn::{BigNum, BigNumContext};
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use serde_json::{json, Value};
use std::fmt;

const COORD_LEN: usize = 32;

/// Private EC P-256 key (PKCS#8 PEM) tagged with a `kid`.
/// `Debug` prints only the `kid`.
#[derive(Clone)]
pub struct SigningKey {
    pkey: PKey<Private>,
    kid: String,
    verifying: VerifyingKey,
}

/// Public EC P-256 key (SPKI PEM) tagged with a `kid`.
#[derive(Clone)]
pub struct VerifyingKey {
    pkey: PKey<Public>,
    kid: String,
    der: Vec<u8>,
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningKey")
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifyingKey")
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}

/// First PEM label (`-----BEGIN <label>-----`) of the input, if any.
fn pem_label(pem: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(pem).ok()?;
    let start = text.find("-----BEGIN ")? + "-----BEGIN ".len();
    let rest = &text[start..];
    let end = rest.find("-----")?;
    Some(&rest[..end])
}

fn ensure_p256(ec: &EcKey<impl openssl::pkey::HasPublic>) -> Result<(), KeyError> {
    if ec.group().curve_name() != Some(Nid::X9_62_PRIME256V1) {
        return Err(KeyError::NotEcP256);
    }
    ec.check_key().map_err(|_| KeyError::InvalidPem)
}

fn check_kid(kid: String) -> Result<String, KeyError> {
    if kid.is_empty() {
        Err(KeyError::EmptyKid)
    } else {
        Ok(kid)
    }
}

impl SigningKey {
    /// Loads a PKCS#8 (`BEGIN PRIVATE KEY`) EC P-256 key.
    pub fn from_pem(pem: impl AsRef<[u8]>, kid: impl Into<String>) -> Result<Self, KeyError> {
        let kid = check_kid(kid.into())?;
        let pem = pem.as_ref();
        match pem_label(pem) {
            Some("PRIVATE KEY") => {}
            Some("EC PRIVATE KEY") | Some("RSA PRIVATE KEY") | Some("ENCRYPTED PRIVATE KEY") => {
                return Err(KeyError::NotPkcs8)
            }
            _ => return Err(KeyError::InvalidPem),
        }
        let pkey = PKey::private_key_from_pem(pem).map_err(|_| KeyError::InvalidPem)?;
        if pkey.id() != Id::EC {
            return Err(KeyError::NotEcP256);
        }
        let ec = pkey.ec_key().map_err(|_| KeyError::NotEcP256)?;
        ensure_p256(&ec)?;
        let der = pkey.public_key_to_der().map_err(|_| KeyError::InvalidPem)?;
        let public = PKey::public_key_from_der(&der).map_err(|_| KeyError::InvalidPem)?;
        let verifying = VerifyingKey {
            pkey: public,
            kid: kid.clone(),
            der,
        };
        Ok(Self {
            pkey,
            kid,
            verifying,
        })
    }

    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// Public half of this key, with the same `kid`.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying.clone()
    }

    pub(crate) fn pkey(&self) -> &PKey<Private> {
        &self.pkey
    }
}

impl VerifyingKey {
    /// Loads an SPKI (`BEGIN PUBLIC KEY`) EC P-256 key.
    pub fn from_pem(pem: impl AsRef<[u8]>, kid: impl Into<String>) -> Result<Self, KeyError> {
        let kid = check_kid(kid.into())?;
        let pem = pem.as_ref();
        match pem_label(pem) {
            Some("PUBLIC KEY") => {}
            Some(_) => return Err(KeyError::NotSpki),
            None => return Err(KeyError::InvalidPem),
        }
        let pkey = PKey::public_key_from_pem(pem).map_err(|_| KeyError::InvalidPem)?;
        if pkey.id() != Id::EC {
            return Err(KeyError::NotEcP256);
        }
        let ec = pkey.ec_key().map_err(|_| KeyError::NotEcP256)?;
        ensure_p256(&ec)?;
        let der = pkey.public_key_to_der().map_err(|_| KeyError::InvalidPem)?;
        Ok(Self { pkey, kid, der })
    }

    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// True when both keys are the same public key (compares the DER, never only the `kid`).
    pub fn same_key(&self, other: &VerifyingKey) -> bool {
        self.der == other.der
    }

    pub(crate) fn pkey(&self) -> &PKey<Public> {
        &self.pkey
    }

    /// Builds a key from one public JWK. Strict: `kty=EC`, `crv=P-256`, non-empty `kid`,
    /// `x`/`y` of 32 bytes on the curve; `use`/`alg`, when present, must be `sig`/`ES256`;
    /// any private parameter (`d`) is refused. Other members are ignored.
    pub fn from_jwk(jwk: &Value) -> Result<Self, KeyError> {
        let obj = jwk.as_object().ok_or(KeyError::InvalidJwks)?;
        if obj.contains_key("d") {
            return Err(KeyError::PrivateKeyInJwks);
        }
        let text = |name: &str| -> Result<Option<&str>, KeyError> {
            match obj.get(name) {
                None => Ok(None),
                Some(Value::String(s)) => Ok(Some(s.as_str())),
                Some(_) => Err(KeyError::InvalidJwks),
            }
        };
        if text("kty")? != Some("EC") || text("crv")? != Some("P-256") {
            return Err(KeyError::NotEcP256);
        }
        if text("use")?.is_some_and(|u| u != "sig") || text("alg")?.is_some_and(|a| a != "ES256") {
            return Err(KeyError::InvalidJwks);
        }
        let kid = check_kid(text("kid")?.unwrap_or_default().to_owned())?;
        let coord = |name: &str| -> Result<BigNum, KeyError> {
            let bytes = text(name)?
                .and_then(b64url_decode)
                .filter(|b| b.len() == COORD_LEN)
                .ok_or(KeyError::InvalidJwks)?;
            BigNum::from_slice(&bytes).map_err(|_| KeyError::InvalidJwks)
        };
        let (x, y) = (coord("x")?, coord("y")?);
        let group =
            EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(|_| KeyError::InvalidJwks)?;
        let ec = EcKey::from_public_key_affine_coordinates(&group, &x, &y)
            .map_err(|_| KeyError::InvalidJwks)?;
        ensure_p256(&ec).map_err(|_| KeyError::InvalidJwks)?;
        let pkey = PKey::from_ec_key(ec).map_err(|_| KeyError::InvalidJwks)?;
        let der = pkey
            .public_key_to_der()
            .map_err(|_| KeyError::InvalidJwks)?;
        Ok(Self { pkey, kid, der })
    }

    fn to_jwk(&self) -> Result<Value, KeyError> {
        let ec = self.pkey.ec_key().map_err(|_| KeyError::NotEcP256)?;
        let mut ctx = BigNumContext::new().map_err(|_| KeyError::InvalidPem)?;
        let mut x = BigNum::new().map_err(|_| KeyError::InvalidPem)?;
        let mut y = BigNum::new().map_err(|_| KeyError::InvalidPem)?;
        ec.public_key()
            .affine_coordinates(ec.group(), &mut x, &mut y, &mut ctx)
            .map_err(|_| KeyError::InvalidPem)?;
        let x = x
            .to_vec_padded(COORD_LEN as i32)
            .map_err(|_| KeyError::InvalidPem)?;
        let y = y
            .to_vec_padded(COORD_LEN as i32)
            .map_err(|_| KeyError::InvalidPem)?;
        Ok(json!({
            "kty": "EC",
            "crv": "P-256",
            "x": b64url_encode(&x),
            "y": b64url_encode(&y),
            "kid": self.kid,
            "use": "sig",
            "alg": "ES256",
        }))
    }
}

/// A set of verifying keys addressed by `kid` (supports rotation).
#[derive(Clone, Debug)]
pub struct VerifyingKeySet {
    pub(crate) keys: Vec<VerifyingKey>,
}

impl VerifyingKeySet {
    /// Fails with `DuplicateKid` if two keys share a `kid`.
    pub fn new(keys: Vec<VerifyingKey>) -> Result<Self, KeyError> {
        for (i, key) in keys.iter().enumerate() {
            if keys[..i].iter().any(|k| k.kid == key.kid) {
                return Err(KeyError::DuplicateKid);
            }
        }
        Ok(Self { keys })
    }

    /// Parses a JWKS document (`{"keys":[...]}`) from JSON text. See [`VerifyingKeySet::from_jwks_value`].
    pub fn from_jwks(jwks: &str) -> Result<Self, KeyError> {
        if jwks.len() > MAX_JWKS_LEN {
            return Err(KeyError::JwksTooLarge);
        }
        let value: Value = serde_json::from_str(jwks).map_err(|_| KeyError::InvalidJwks)?;
        Self::from_jwks_value(&value)
    }

    /// Parses a JWKS document from a JSON value. Strict: only EC P-256 public keys with a
    /// `kid` (see [`VerifyingKey::from_jwk`]); a key of another type/curve, a private
    /// parameter, a duplicate `kid`, an empty set or an oversized document is an error,
    /// never silently ignored. Round-trips with [`VerifyingKeySet::to_jwks`].
    pub fn from_jwks_value(jwks: &Value) -> Result<Self, KeyError> {
        if jwks.to_string().len() > MAX_JWKS_LEN {
            return Err(KeyError::JwksTooLarge);
        }
        let list = jwks
            .get("keys")
            .and_then(Value::as_array)
            .ok_or(KeyError::InvalidJwks)?;
        if list.is_empty() {
            return Err(KeyError::EmptyKeySet);
        }
        let keys = list
            .iter()
            .map(VerifyingKey::from_jwk)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(keys)
    }

    pub fn get(&self, kid: &str) -> Option<&VerifyingKey> {
        self.keys.iter().find(|k| k.kid == kid)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Public JWKS document: `{"keys":[{"kty":"EC","crv":"P-256",...}]}`. Never contains private material.
    pub fn to_jwks(&self) -> Value {
        let keys: Vec<Value> = self.keys.iter().filter_map(|k| k.to_jwk().ok()).collect();
        json!({ "keys": keys })
    }
}

/// Generates an EC P-256 key pair: `(private PKCS#8 PEM, public SPKI PEM)`.
pub fn generate_ec_p256_keypair() -> Result<(String, String), KeyError> {
    let group =
        EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(|_| KeyError::GenerationFailed)?;
    let ec = EcKey::generate(&group).map_err(|_| KeyError::GenerationFailed)?;
    let pkey = PKey::from_ec_key(ec).map_err(|_| KeyError::GenerationFailed)?;
    let private = pkey
        .private_key_to_pem_pkcs8()
        .map_err(|_| KeyError::GenerationFailed)?;
    let public = pkey
        .public_key_to_pem()
        .map_err(|_| KeyError::GenerationFailed)?;
    Ok((
        String::from_utf8(private).map_err(|_| KeyError::GenerationFailed)?,
        String::from_utf8(public).map_err(|_| KeyError::GenerationFailed)?,
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::util::b64url_decode;
    use openssl::rsa::Rsa;

    fn pair() -> (String, String) {
        generate_ec_p256_keypair().unwrap()
    }

    fn ec_pkcs8(nid: Nid) -> String {
        let group = EcGroup::from_curve_name(nid).unwrap();
        let pkey = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        String::from_utf8(pkey.private_key_to_pem_pkcs8().unwrap()).unwrap()
    }

    fn ec_spki(nid: Nid) -> String {
        let group = EcGroup::from_curve_name(nid).unwrap();
        let pkey = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        String::from_utf8(pkey.public_key_to_pem().unwrap()).unwrap()
    }

    #[test]
    fn round_trip_private_to_public() {
        let (private, public) = pair();
        let signing = SigningKey::from_pem(&private, "k1").unwrap();
        let verifying = VerifyingKey::from_pem(&public, "k1").unwrap();
        assert!(signing.verifying_key().same_key(&verifying));
        assert_eq!(signing.kid(), "k1");
        assert_eq!(signing.verifying_key().kid(), "k1");
        assert_eq!(verifying.kid(), "k1");
    }

    #[test]
    fn rsa_is_rejected() {
        let rsa = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let private = rsa.private_key_to_pem_pkcs8().unwrap();
        let public = rsa.public_key_to_pem().unwrap();
        assert_eq!(
            SigningKey::from_pem(private, "k").unwrap_err(),
            KeyError::NotEcP256
        );
        assert_eq!(
            VerifyingKey::from_pem(public, "k").unwrap_err(),
            KeyError::NotEcP256
        );
    }

    #[test]
    fn other_curves_are_rejected() {
        for nid in [Nid::SECP384R1, Nid::SECP256K1] {
            assert_eq!(
                SigningKey::from_pem(ec_pkcs8(nid), "k").unwrap_err(),
                KeyError::NotEcP256
            );
            assert_eq!(
                VerifyingKey::from_pem(ec_spki(nid), "k").unwrap_err(),
                KeyError::NotEcP256
            );
        }
    }

    #[test]
    fn sec1_private_key_is_rejected() {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let sec1 = EcKey::generate(&group)
            .unwrap()
            .private_key_to_pem()
            .unwrap();
        assert!(String::from_utf8_lossy(&sec1).contains("BEGIN EC PRIVATE KEY"));
        assert_eq!(
            SigningKey::from_pem(sec1, "k").unwrap_err(),
            KeyError::NotPkcs8
        );
    }

    #[test]
    fn swapped_pem_kinds_are_rejected() {
        let (private, public) = pair();
        assert!(SigningKey::from_pem(&public, "k").is_err());
        assert!(VerifyingKey::from_pem(&private, "k").is_err());
    }

    #[test]
    fn garbage_and_empty_are_rejected() {
        for input in [
            "",
            "garbage",
            "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n",
        ] {
            assert!(SigningKey::from_pem(input, "k").is_err());
        }
        for input in [
            "",
            "garbage",
            "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----\n",
        ] {
            assert!(VerifyingKey::from_pem(input, "k").is_err());
        }
    }

    #[test]
    fn empty_kid_is_rejected() {
        let (private, public) = pair();
        assert_eq!(
            SigningKey::from_pem(&private, "").unwrap_err(),
            KeyError::EmptyKid
        );
        assert_eq!(
            VerifyingKey::from_pem(&public, "").unwrap_err(),
            KeyError::EmptyKid
        );
    }

    #[test]
    fn duplicate_kid_is_rejected() {
        let (_, a) = pair();
        let (_, b) = pair();
        let keys = vec![
            VerifyingKey::from_pem(&a, "same").unwrap(),
            VerifyingKey::from_pem(&b, "same").unwrap(),
        ];
        assert_eq!(
            VerifyingKeySet::new(keys).unwrap_err(),
            KeyError::DuplicateKid
        );
    }

    #[test]
    fn same_key_compares_the_key_not_the_kid() {
        let (_, a) = pair();
        let (_, b) = pair();
        let a1 = VerifyingKey::from_pem(&a, "one").unwrap();
        let a2 = VerifyingKey::from_pem(&a, "two").unwrap();
        let b1 = VerifyingKey::from_pem(&b, "one").unwrap();
        assert!(a1.same_key(&a2));
        assert!(!a1.same_key(&b1));
    }

    #[test]
    fn set_lookup_by_kid() {
        let (_, a) = pair();
        let (_, b) = pair();
        let set = VerifyingKeySet::new(vec![
            VerifyingKey::from_pem(&a, "a").unwrap(),
            VerifyingKey::from_pem(&b, "b").unwrap(),
        ])
        .unwrap();
        assert_eq!(set.len(), 2);
        assert!(set.get("a").is_some());
        assert!(set.get("c").is_none());
    }

    #[test]
    fn jwks_has_exactly_the_expected_fields() {
        let (private, public) = pair();
        let signing = SigningKey::from_pem(&private, "kid-1").unwrap();
        let set = VerifyingKeySet::new(vec![signing.verifying_key()]).unwrap();
        let jwks = set.to_jwks();
        let keys = jwks["keys"].as_array().unwrap();
        assert_eq!(keys.len(), 1);
        let jwk = keys[0].as_object().unwrap();
        let mut names: Vec<&str> = jwk.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["alg", "crv", "kid", "kty", "use", "x", "y"]);
        assert_eq!(jwk["kty"], "EC");
        assert_eq!(jwk["crv"], "P-256");
        assert_eq!(jwk["use"], "sig");
        assert_eq!(jwk["alg"], "ES256");
        assert_eq!(jwk["kid"], "kid-1");
        for c in ["x", "y"] {
            let s = jwk[c].as_str().unwrap();
            assert_eq!(s.len(), 43);
            assert_eq!(b64url_decode(s).unwrap().len(), 32);
        }
        assert!(!jwks.to_string().contains("\"d\""));
        assert!(crate::cryptox::jose::jwk_is_valid_p256(&keys[0]));
        let _ = public;
    }

    fn jwk_of(kid: &str) -> Value {
        let (_, public) = pair();
        VerifyingKeySet::new(vec![VerifyingKey::from_pem(public, kid).unwrap()])
            .unwrap()
            .to_jwks()["keys"][0]
            .clone()
    }

    fn set_of(keys: Vec<Value>) -> Value {
        json!({ "keys": keys })
    }

    #[test]
    fn jwks_round_trip() {
        let (_, a) = pair();
        let (_, b) = pair();
        let set = VerifyingKeySet::new(vec![
            VerifyingKey::from_pem(&a, "a").unwrap(),
            VerifyingKey::from_pem(&b, "b").unwrap(),
        ])
        .unwrap();
        let jwks = set.to_jwks();
        let from_value = VerifyingKeySet::from_jwks_value(&jwks).unwrap();
        let from_text = VerifyingKeySet::from_jwks(&jwks.to_string()).unwrap();
        assert_eq!(from_value.to_jwks(), jwks);
        assert_eq!(from_text.to_jwks(), jwks);
        assert!(from_text
            .get("a")
            .unwrap()
            .same_key(&VerifyingKey::from_pem(&a, "x").unwrap()));
        // use/alg are optional
        let mut bare = jwk_of("k");
        bare.as_object_mut().unwrap().remove("use");
        bare.as_object_mut().unwrap().remove("alg");
        assert!(VerifyingKeySet::from_jwks_value(&set_of(vec![bare])).is_ok());
    }

    #[test]
    fn jwks_rejects_private_material() {
        let mut jwk = jwk_of("k");
        jwk["d"] = json!("c2VjcmV0");
        assert_eq!(
            VerifyingKeySet::from_jwks_value(&set_of(vec![jwk])).unwrap_err(),
            KeyError::PrivateKeyInJwks
        );
    }

    #[test]
    fn jwks_rejects_other_key_types_and_curves_instead_of_ignoring() {
        let mut other_kty = jwk_of("k");
        other_kty["kty"] = json!("RSA");
        let mut other_crv = jwk_of("k");
        other_crv["crv"] = json!("P-384");
        for bad in [other_kty, other_crv] {
            // mixed with a valid key: must still fail, not skip
            let doc = set_of(vec![jwk_of("good"), bad]);
            assert_eq!(
                VerifyingKeySet::from_jwks_value(&doc).unwrap_err(),
                KeyError::NotEcP256
            );
        }
    }

    #[test]
    fn jwks_rejects_missing_kid_bad_use_alg_and_bad_coordinates() {
        let mut no_kid = jwk_of("k");
        no_kid.as_object_mut().unwrap().remove("kid");
        let mut empty_kid = jwk_of("k");
        empty_kid["kid"] = json!("");
        for bad in [no_kid, empty_kid] {
            assert_eq!(
                VerifyingKeySet::from_jwks_value(&set_of(vec![bad])).unwrap_err(),
                KeyError::EmptyKid
            );
        }
        let mut enc = jwk_of("k");
        enc["use"] = json!("enc");
        let mut alg = jwk_of("k");
        alg["alg"] = json!("RS256");
        let mut short = jwk_of("k");
        short["x"] = json!("AAAA");
        let mut off_curve = jwk_of("k");
        off_curve["y"] = json!(b64url_encode(&[1u8; 32]));
        let mut not_text = jwk_of("k");
        not_text["x"] = json!(7);
        for bad in [enc, alg, short, off_curve, not_text] {
            assert_eq!(
                VerifyingKeySet::from_jwks_value(&set_of(vec![bad])).unwrap_err(),
                KeyError::InvalidJwks
            );
        }
    }

    #[test]
    fn jwks_rejects_duplicate_kid_empty_set_malformed_and_oversized() {
        let doc = set_of(vec![jwk_of("same"), jwk_of("same")]);
        assert_eq!(
            VerifyingKeySet::from_jwks_value(&doc).unwrap_err(),
            KeyError::DuplicateKid
        );
        assert_eq!(
            VerifyingKeySet::from_jwks("{\"keys\":[]}").unwrap_err(),
            KeyError::EmptyKeySet
        );
        for bad in [
            "",
            "not json",
            "[]",
            "{}",
            "{\"keys\":{}}",
            "{\"keys\":[1]}",
        ] {
            assert_eq!(
                VerifyingKeySet::from_jwks(bad).unwrap_err(),
                KeyError::InvalidJwks,
                "{bad}"
            );
        }
        let big = format!("{{\"keys\":[],\"pad\":\"{}\"}}", "a".repeat(MAX_JWKS_LEN));
        assert_eq!(
            VerifyingKeySet::from_jwks(&big).unwrap_err(),
            KeyError::JwksTooLarge
        );
        let big_value: Value = serde_json::from_str(&big).unwrap();
        assert_eq!(
            VerifyingKeySet::from_jwks_value(&big_value).unwrap_err(),
            KeyError::JwksTooLarge
        );
    }

    #[test]
    fn jwks_errors_do_not_leak_the_document() {
        let mut jwk = jwk_of("secret-kid-marker");
        jwk["d"] = json!("PRIVATE-MARKER");
        let err = VerifyingKeySet::from_jwks(&set_of(vec![jwk]).to_string()).unwrap_err();
        let text = format!("{err} {err:?}");
        assert!(!text.contains("PRIVATE-MARKER") && !text.contains("secret-kid-marker"));
    }

    #[test]
    fn debug_shows_kid_only() {
        let (private, _) = pair();
        let signing = SigningKey::from_pem(&private, "my-kid").unwrap();
        let dbg = format!("{signing:?} {:?}", signing.verifying_key());
        assert!(dbg.contains("my-kid"));
        let body: String = private
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();
        assert!(!dbg.contains(&body[..20]));
        assert!(!dbg.contains("BEGIN"));
    }

    #[test]
    fn key_error_display_does_not_contain_pem() {
        let (private, _) = pair();
        let body: String = private
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();
        let err = VerifyingKey::from_pem(&private, "k").unwrap_err();
        assert!(!err.to_string().contains(&body[..20]));
        assert!(!format!("{err:?}").contains(&body[..20]));
    }

    #[test]
    fn generated_pems_have_expected_labels() {
        let (private, public) = pair();
        assert!(private.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(public.starts_with("-----BEGIN PUBLIC KEY-----"));
    }
}
