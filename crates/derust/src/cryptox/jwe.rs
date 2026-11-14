//! Compact JWE (`ECDH-ES` direct key agreement + `A256GCM`) for EC P-256 keys.
//!
//! Every `seal` generates a fresh ephemeral key (`epk`). `open` validates the
//! protected header with its own parser before and after calling the backend, and
//! reports every cryptographic failure as the single `DecryptFailed` error.

use crate::cryptox::error::JweError;
use crate::cryptox::jose::{jwe_decrypt_ecdh_es, jwe_encrypt_ecdh_es_a256gcm};
use crate::cryptox::keys::{SigningKey, VerifyingKey};
use crate::cryptox::util::{b64url_decode, MAX_JWE_LEN};
use serde_json::Value;
use std::fmt;
use zeroize::Zeroizing;

const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// Maximum length (bytes) of a JWE `kid`.
pub const MAX_JWE_KID_LEN: usize = 128;

/// Anything that can seal data into a JWE. Services depend on this trait so tests can inject fakes.
pub trait Sealer: Send + Sync {
    fn seal(&self, plaintext: &[u8]) -> Result<String, JweError>;
}

/// Seals data to an EC P-256 public key (SPKI PEM).
#[derive(Clone)]
pub struct JweSealer {
    public_pem: Vec<u8>,
    kid: Option<String>,
}

impl JweSealer {
    /// Fails with `InvalidKey` unless `public_pem` is an SPKI EC P-256 public key.
    pub fn new(public_pem: impl AsRef<[u8]>) -> Result<Self, JweError> {
        let key = VerifyingKey::from_pem(public_pem, "jwe").map_err(|_| JweError::InvalidKey)?;
        let public_pem = key
            .pkey()
            .public_key_to_pem()
            .map_err(|_| JweError::InvalidKey)?;
        Ok(Self {
            public_pem,
            kid: None,
        })
    }

    /// Puts `kid` in the protected header so the recipient can pick the right key during
    /// rotation (see [`JweOpenerSet`]). Fails with `InvalidKey` if empty or longer than
    /// [`MAX_JWE_KID_LEN`] bytes. Without it no `kid` is emitted (default).
    pub fn with_kid(mut self, kid: impl Into<String>) -> Result<Self, JweError> {
        let kid = kid.into();
        if kid.is_empty() || kid.len() > MAX_JWE_KID_LEN {
            return Err(JweError::InvalidKey);
        }
        self.kid = Some(kid);
        Ok(self)
    }
}

impl fmt::Debug for JweSealer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JweSealer(***)")
    }
}

impl Sealer for JweSealer {
    /// The output is refused (`SealFailed`) if it would exceed what [`JweOpener::open`] accepts.
    fn seal(&self, plaintext: &[u8]) -> Result<String, JweError> {
        let jwe = jwe_encrypt_ecdh_es_a256gcm(&self.public_pem, plaintext, self.kid.as_deref())
            .map_err(|_| JweError::SealFailed)?;
        if jwe.len() > MAX_JWE_LEN {
            return Err(JweError::SealFailed);
        }
        Ok(jwe)
    }
}

/// Opens JWEs sealed to the matching public key (PKCS#8 PEM private key).
pub struct JweOpener {
    private_pem: Zeroizing<Vec<u8>>,
}

impl JweOpener {
    /// Fails with `InvalidKey` unless `private_pem` is a PKCS#8 EC P-256 private key.
    pub fn new(private_pem: impl AsRef<[u8]>) -> Result<Self, JweError> {
        let key = SigningKey::from_pem(private_pem, "jwe").map_err(|_| JweError::InvalidKey)?;
        let pem = key
            .pkey()
            .private_key_to_pem_pkcs8()
            .map_err(|_| JweError::InvalidKey)?;
        Ok(Self {
            private_pem: Zeroizing::new(pem),
        })
    }

    /// Opens a JWE with or without `kid` (a present `kid` is ignored here; use
    /// [`JweOpenerSet`] to select a key by it).
    pub fn open(&self, jwe: &str) -> Result<Vec<u8>, JweError> {
        validate_compact(jwe)?;
        jwe_decrypt_ecdh_es(&self.private_pem, jwe).map_err(|_| JweError::DecryptFailed)
    }
}

/// Several recipient keys addressed by `kid` (rotation of the recipient key).
///
/// The `kid` is read from the protected header only to choose the key, BEFORE decrypting;
/// nothing else in the header is trusted. A missing or unknown `kid`, and every
/// cryptographic failure, is the single `DecryptFailed` (no error oracle). A `kid` longer
/// than [`MAX_JWE_KID_LEN`] or not a string is `Malformed` (structural, pre-crypto).
pub struct JweOpenerSet {
    openers: Vec<(String, JweOpener)>,
}

impl JweOpenerSet {
    /// Fails with `InvalidKey` for an empty set, an empty/oversized or duplicate `kid`.
    pub fn new(openers: Vec<(String, JweOpener)>) -> Result<Self, JweError> {
        if openers.is_empty() {
            return Err(JweError::InvalidKey);
        }
        for (i, (kid, _)) in openers.iter().enumerate() {
            if kid.is_empty()
                || kid.len() > MAX_JWE_KID_LEN
                || openers[..i].iter().any(|(k, _)| k == kid)
            {
                return Err(JweError::InvalidKey);
            }
        }
        Ok(Self { openers })
    }

    pub fn len(&self) -> usize {
        self.openers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.openers.is_empty()
    }

    pub fn open(&self, jwe: &str) -> Result<Vec<u8>, JweError> {
        let kid = validate_compact(jwe)?;
        let opener = kid
            .as_deref()
            .and_then(|kid| self.openers.iter().find(|(k, _)| k == kid))
            .map(|(_, o)| o)
            .ok_or(JweError::DecryptFailed)?;
        opener.open(jwe)
    }
}

impl fmt::Debug for JweOpenerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JweOpenerSet(***)")
    }
}

impl fmt::Debug for JweOpener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JweOpener(***)")
    }
}

/// Validates the compact JWE and returns its `kid`, if any.
fn validate_compact(jwe: &str) -> Result<Option<String>, JweError> {
    if jwe.len() > MAX_JWE_LEN {
        return Err(JweError::Malformed);
    }
    let segments: Vec<&str> = jwe.split('.').collect();
    if segments.len() != 5 {
        return Err(JweError::Malformed);
    }
    let decoded: Vec<Vec<u8>> = segments
        .iter()
        .map(|s| b64url_decode(s))
        .collect::<Option<_>>()
        .ok_or(JweError::Malformed)?;
    let header = match serde_json::from_slice::<Value>(&decoded[0]) {
        Ok(Value::Object(map)) => map,
        _ => return Err(JweError::Malformed),
    };
    if header.get("alg").and_then(Value::as_str) != Some("ECDH-ES")
        || header.get("enc").and_then(Value::as_str) != Some("A256GCM")
    {
        return Err(JweError::UnsupportedAlgorithm);
    }
    if header.contains_key("zip") || header.contains_key("crit") {
        return Err(JweError::Malformed);
    }
    let epk_ok = header.get("epk").is_some_and(|epk| {
        epk.get("kty").and_then(Value::as_str) == Some("EC")
            && epk.get("crv").and_then(Value::as_str) == Some("P-256")
    });
    if !epk_ok {
        return Err(JweError::Malformed);
    }
    if !decoded[1].is_empty() || decoded[2].len() != IV_LEN || decoded[4].len() != TAG_LEN {
        return Err(JweError::Malformed);
    }
    match header.get("kid") {
        None => Ok(None),
        Some(Value::String(kid)) if kid.len() <= MAX_JWE_KID_LEN => Ok(Some(kid.clone())),
        Some(_) => Err(JweError::Malformed),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cryptox::keys::generate_ec_p256_keypair;
    use crate::cryptox::util::b64url_encode;
    use josekit::jwe::{JweHeader, ECDH_ES, ECDH_ES_A256KW};
    use openssl::ec::{EcGroup, EcKey};
    use openssl::nid::Nid;
    use openssl::pkey::PKey;

    fn pair() -> (JweSealer, JweOpener, String, String) {
        let (private, public) = generate_ec_p256_keypair().unwrap();
        (
            JweSealer::new(&public).unwrap(),
            JweOpener::new(&private).unwrap(),
            private,
            public,
        )
    }

    #[test]
    fn round_trip_various_payloads() {
        let (sealer, opener, _, _) = pair();
        let binary: Vec<u8> = (0..=255u8).collect();
        let big = vec![0xAB; 40 * 1024];
        for data in [&b""[..], b"x", b"hello world", &binary, &big] {
            let jwe = sealer.seal(data).unwrap();
            assert_eq!(jwe.split('.').count(), 5);
            assert_eq!(opener.open(&jwe).unwrap(), data);
        }
    }

    #[test]
    fn seals_differ_and_use_a_fresh_epk() {
        let (sealer, opener, _, _) = pair();
        let a = sealer.seal(b"same").unwrap();
        let b = sealer.seal(b"same").unwrap();
        assert_ne!(a, b);
        let epk = |jwe: &str| {
            let h: Value =
                serde_json::from_slice(&b64url_decode(jwe.split('.').next().unwrap()).unwrap())
                    .unwrap();
            h["epk"].clone()
        };
        assert_ne!(epk(&a), epk(&b));
        assert_eq!(opener.open(&a).unwrap(), b"same");
        assert_eq!(opener.open(&b).unwrap(), b"same");
    }

    #[test]
    fn other_private_key_fails_to_decrypt() {
        let (sealer, _, _, _) = pair();
        let (_, other, _, _) = pair();
        let jwe = sealer.seal(b"secret").unwrap();
        assert_eq!(other.open(&jwe).unwrap_err(), JweError::DecryptFailed);
    }

    #[test]
    fn tampering_any_segment_fails() {
        let (sealer, opener, _, _) = pair();
        let jwe = sealer.seal(b"secret payload").unwrap();
        let parts: Vec<&str> = jwe.split('.').collect();
        for idx in 0..5 {
            let mut bytes = b64url_decode(parts[idx]).unwrap();
            if bytes.is_empty() {
                bytes.push(1);
            } else {
                let last = bytes.len() - 1;
                bytes[last] ^= 1;
            }
            let mut p: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
            p[idx] = b64url_encode(&bytes);
            assert!(opener.open(&p.join(".")).is_err(), "segment {idx}");
        }
    }

    fn josekit_jwe(alg_a256kw: bool, enc: &str, zip: bool, public_pem: &[u8]) -> String {
        let mut header = JweHeader::new();
        header.set_content_encryption(enc);
        if zip {
            header.set_compression("DEF");
        }
        if alg_a256kw {
            let enc = ECDH_ES_A256KW.encrypter_from_pem(public_pem).unwrap();
            josekit::jwe::serialize_compact(b"data", &header, &enc).unwrap()
        } else {
            let enc = ECDH_ES.encrypter_from_pem(public_pem).unwrap();
            josekit::jwe::serialize_compact(b"data", &header, &enc).unwrap()
        }
    }

    #[test]
    fn other_algorithms_are_rejected() {
        let (_, opener, _, public) = pair();
        let p = public.as_bytes();
        assert_eq!(
            opener
                .open(&josekit_jwe(true, "A256GCM", false, p))
                .unwrap_err(),
            JweError::UnsupportedAlgorithm
        );
        for enc in ["A128GCM", "A256CBC-HS512"] {
            assert_eq!(
                opener.open(&josekit_jwe(false, enc, false, p)).unwrap_err(),
                JweError::UnsupportedAlgorithm,
                "{enc}"
            );
        }
        for alg in ["RSA-OAEP", "dir"] {
            let h = format!(r#"{{"alg":"{alg}","enc":"A256GCM"}}"#);
            let jwe = format!(
                "{}.AAAA.AAAAAAAAAAAAAAAA.AAAA.AAAAAAAAAAAAAAAAAAAAAA",
                b64url_encode(h.as_bytes())
            );
            assert_eq!(
                opener.open(&jwe).unwrap_err(),
                JweError::UnsupportedAlgorithm,
                "{alg}"
            );
        }
    }

    #[test]
    fn zip_is_rejected() {
        let (_, opener, _, public) = pair();
        let jwe = josekit_jwe(false, "A256GCM", true, public.as_bytes());
        assert_eq!(opener.open(&jwe).unwrap_err(), JweError::Malformed);
    }

    #[test]
    fn epk_on_another_curve_is_rejected() {
        let (_, opener, _, _) = pair();
        let group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
        let pkey = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let public = pkey.public_key_to_pem().unwrap();
        let jwe = josekit_jwe(false, "A256GCM", false, &public);
        assert_eq!(opener.open(&jwe).unwrap_err(), JweError::Malformed);
    }

    #[test]
    fn malformed_inputs() {
        let (_, opener, _, _) = pair();
        let huge = "a".repeat(MAX_JWE_LEN + 1);
        for t in ["", "a.b.c.d", "a.b.c.d.e.f", "!!.b.c.d.e", huge.as_str()] {
            assert_eq!(opener.open(t).unwrap_err(), JweError::Malformed, "{t:.20}");
        }
    }

    #[test]
    fn invalid_keys_are_refused() {
        let rsa = openssl::rsa::Rsa::generate(2048).unwrap();
        let rsa_key = PKey::from_rsa(rsa).unwrap();
        let rsa_private = rsa_key.private_key_to_pem_pkcs8().unwrap();
        let rsa_public = rsa_key.public_key_to_pem().unwrap();
        let group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
        let p384 = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        assert_eq!(
            JweSealer::new(rsa_public).unwrap_err(),
            JweError::InvalidKey
        );
        assert_eq!(
            JweSealer::new(p384.public_key_to_pem().unwrap()).unwrap_err(),
            JweError::InvalidKey
        );
        assert_eq!(
            JweOpener::new(rsa_private).unwrap_err(),
            JweError::InvalidKey
        );
        assert_eq!(
            JweOpener::new(p384.private_key_to_pem_pkcs8().unwrap()).unwrap_err(),
            JweError::InvalidKey
        );
        assert_eq!(JweOpener::new("junk").unwrap_err(), JweError::InvalidKey);
    }

    struct FakeSealer;
    impl Sealer for FakeSealer {
        fn seal(&self, plaintext: &[u8]) -> Result<String, JweError> {
            Ok(format!("fake:{}", plaintext.len()))
        }
    }

    fn use_generic(s: &impl Sealer) -> String {
        s.seal(b"abc").unwrap()
    }

    #[test]
    fn sealer_trait_is_injectable() {
        let dynamic: &dyn Sealer = &FakeSealer;
        assert_eq!(dynamic.seal(b"ab").unwrap(), "fake:2");
        assert_eq!(use_generic(&FakeSealer), "fake:3");
        let (sealer, _, _, _) = pair();
        let boxed: Box<dyn Sealer> = Box::new(sealer);
        assert!(boxed.seal(b"x").is_ok());
    }

    #[test]
    fn debug_and_errors_do_not_leak() {
        let (sealer, opener, private, public) = pair();
        let body: String = private.lines().nth(1).unwrap().to_owned();
        assert!(!format!("{opener:?}").contains(&body));
        assert!(!format!("{sealer:?}").contains(public.lines().nth(1).unwrap()));
        let jwe = sealer.seal(b"PLAINTEXT-MARKER").unwrap();
        let tampered = format!("{jwe}x");
        let err = opener.open(&tampered).unwrap_err();
        let text = format!("{err:?} {err}");
        assert!(
            !text.contains("PLAINTEXT-MARKER")
                && !text.contains(&tampered)
                && !text.contains(&body)
        );
        let err = JweOpener::new(private.replace(&body, "AAAA")).unwrap_err();
        assert!(!format!("{err:?} {err}").contains("AAAA"));
    }

    #[test]
    fn oversized_seal_is_refused() {
        let (sealer, _, _, _) = pair();
        assert_eq!(
            sealer.seal(&vec![0u8; MAX_JWE_LEN]).unwrap_err(),
            JweError::SealFailed
        );
    }

    // Interoperability with an independent implementation (Node `jose`, see tests/cryptox/README.md).
    const NODE_PRIVATE: &str = include_str!("../../tests/cryptox/node_jose.private.pem");
    const NODE_PUBLIC: &str = include_str!("../../tests/cryptox/node_jose.public.pem");
    const NODE_JWE: &str = include_str!("../../tests/cryptox/node_jose.jwe");
    const NODE_PLAINTEXT: &str = include_str!("../../tests/cryptox/node_jose.plaintext");

    #[test]
    fn opens_jwe_produced_by_node_jose() {
        let opener = JweOpener::new(NODE_PRIVATE).unwrap();
        let plain = opener.open(NODE_JWE.trim()).unwrap();
        assert_eq!(plain, NODE_PLAINTEXT.as_bytes());
    }

    #[test]
    fn writes_seal_for_node_jose_to_open_when_requested() {
        // Reverse direction: set CRYPTOX_JWE_OUT to a path; tests/cryptox/README.md explains
        // how to open that file with Node `jose`. Without the variable this only round-trips.
        let sealer = JweSealer::new(NODE_PUBLIC).unwrap();
        let jwe = sealer.seal(NODE_PLAINTEXT.as_bytes()).unwrap();
        assert_eq!(
            JweOpener::new(NODE_PRIVATE).unwrap().open(&jwe).unwrap(),
            NODE_PLAINTEXT.as_bytes()
        );
        if let Ok(path) = std::env::var("CRYPTOX_JWE_OUT") {
            std::fs::write(path, jwe).unwrap();
        }
    }

    const NODE_KID_JWE: &str = include_str!("../../tests/cryptox/node_jose_kid.jwe");

    #[test]
    fn opens_jwe_with_kid_produced_by_node_jose() {
        assert_eq!(header_of(NODE_KID_JWE.trim())["kid"], "node-kid-1");
        let opener = || JweOpener::new(NODE_PRIVATE).unwrap();
        assert_eq!(
            opener().open(NODE_KID_JWE.trim()).unwrap(),
            NODE_PLAINTEXT.as_bytes()
        );
        let set = JweOpenerSet::new(vec![("node-kid-1".into(), opener())]).unwrap();
        assert_eq!(
            set.open(NODE_KID_JWE.trim()).unwrap(),
            NODE_PLAINTEXT.as_bytes()
        );
        let other = JweOpenerSet::new(vec![("other".into(), opener())]).unwrap();
        assert_eq!(
            other.open(NODE_KID_JWE.trim()).unwrap_err(),
            JweError::DecryptFailed
        );
    }

    #[test]
    fn writes_seal_with_kid_for_node_jose_to_open_when_requested() {
        let sealer = JweSealer::new(NODE_PUBLIC)
            .unwrap()
            .with_kid("derust-kid-1")
            .unwrap();
        let jwe = sealer.seal(NODE_PLAINTEXT.as_bytes()).unwrap();
        assert_eq!(
            JweOpener::new(NODE_PRIVATE).unwrap().open(&jwe).unwrap(),
            NODE_PLAINTEXT.as_bytes()
        );
        if let Ok(path) = std::env::var("CRYPTOX_JWE_KID_OUT") {
            std::fs::write(path, jwe).unwrap();
        }
    }

    fn header_of(jwe: &str) -> Value {
        serde_json::from_slice(&b64url_decode(jwe.split('.').next().unwrap()).unwrap()).unwrap()
    }

    #[test]
    fn kid_is_in_the_protected_header_only_when_requested() {
        let (private, public) = generate_ec_p256_keypair().unwrap();
        let opener = JweOpener::new(&private).unwrap();
        let plain = JweSealer::new(&public).unwrap().seal(b"x").unwrap();
        assert!(header_of(&plain).get("kid").is_none());
        let tagged = JweSealer::new(&public)
            .unwrap()
            .with_kid("k-2026")
            .unwrap()
            .seal(b"x")
            .unwrap();
        assert_eq!(header_of(&tagged)["kid"], "k-2026");
        // a plain opener opens both, with and without kid
        assert_eq!(opener.open(&plain).unwrap(), b"x");
        assert_eq!(opener.open(&tagged).unwrap(), b"x");
    }

    #[test]
    fn with_kid_validates_length_and_emptiness() {
        let (_, public) = generate_ec_p256_keypair().unwrap();
        let s = || JweSealer::new(&public).unwrap();
        assert_eq!(s().with_kid("").unwrap_err(), JweError::InvalidKey);
        assert_eq!(
            s().with_kid("a".repeat(MAX_JWE_KID_LEN + 1)).unwrap_err(),
            JweError::InvalidKey
        );
        assert!(s().with_kid("a".repeat(MAX_JWE_KID_LEN)).is_ok());
    }

    fn rotation() -> (JweSealer, JweSealer, JweOpenerSet) {
        let (p1, u1) = generate_ec_p256_keypair().unwrap();
        let (p2, u2) = generate_ec_p256_keypair().unwrap();
        let set = JweOpenerSet::new(vec![
            ("old".into(), JweOpener::new(&p1).unwrap()),
            ("new".into(), JweOpener::new(&p2).unwrap()),
        ])
        .unwrap();
        (
            JweSealer::new(&u1).unwrap().with_kid("old").unwrap(),
            JweSealer::new(&u2).unwrap().with_kid("new").unwrap(),
            set,
        )
    }

    #[test]
    fn opener_set_picks_the_key_by_kid() {
        let (old, new, set) = rotation();
        assert_eq!(set.len(), 2);
        assert_eq!(set.open(&old.seal(b"a").unwrap()).unwrap(), b"a");
        assert_eq!(set.open(&new.seal(b"b").unwrap()).unwrap(), b"b");
    }

    #[test]
    fn opener_set_missing_unknown_or_wrong_kid_is_decrypt_failed() {
        let (old, _, set) = rotation();
        let (_, public) = generate_ec_p256_keypair().unwrap();
        let unknown = JweSealer::new(&public).unwrap().with_kid("zzz").unwrap();
        let no_kid = JweSealer::new(&public).unwrap();
        assert_eq!(
            set.open(&unknown.seal(b"a").unwrap()).unwrap_err(),
            JweError::DecryptFailed
        );
        assert_eq!(
            set.open(&no_kid.seal(b"a").unwrap()).unwrap_err(),
            JweError::DecryptFailed
        );
        // known kid but sealed to a different key
        let liar = JweSealer::new(&public).unwrap().with_kid("old").unwrap();
        assert_eq!(
            set.open(&liar.seal(b"a").unwrap()).unwrap_err(),
            JweError::DecryptFailed
        );
        // tampered ciphertext with a good kid
        let jwe = old.seal(b"secret").unwrap();
        let mut parts: Vec<String> = jwe.split('.').map(str::to_owned).collect();
        let mut ct = b64url_decode(&parts[3]).unwrap();
        ct[0] ^= 1;
        parts[3] = b64url_encode(&ct);
        assert_eq!(
            set.open(&parts.join(".")).unwrap_err(),
            JweError::DecryptFailed
        );
    }

    #[test]
    fn opener_set_rejects_oversized_or_non_string_kid_before_lookup() {
        let (_, _, set) = rotation();
        let (_, public) = generate_ec_p256_keypair().unwrap();
        let jwe = JweSealer::new(&public).unwrap().seal(b"x").unwrap();
        let rewrite = |kid: Value| {
            let mut h = header_of(&jwe);
            h["kid"] = kid;
            let mut parts: Vec<String> = jwe.split('.').map(str::to_owned).collect();
            parts[0] = b64url_encode(h.to_string().as_bytes());
            parts.join(".")
        };
        for kid in [
            serde_json::json!("a".repeat(MAX_JWE_KID_LEN + 1)),
            serde_json::json!(7),
        ] {
            assert_eq!(set.open(&rewrite(kid)).unwrap_err(), JweError::Malformed);
        }
    }

    #[test]
    fn opener_set_construction_rules_and_debug() {
        let op = || JweOpener::new(generate_ec_p256_keypair().unwrap().0).unwrap();
        assert_eq!(JweOpenerSet::new(vec![]).unwrap_err(), JweError::InvalidKey);
        assert_eq!(
            JweOpenerSet::new(vec![("".into(), op())]).unwrap_err(),
            JweError::InvalidKey
        );
        assert_eq!(
            JweOpenerSet::new(vec![("a".into(), op()), ("a".into(), op())]).unwrap_err(),
            JweError::InvalidKey
        );
        let set = JweOpenerSet::new(vec![("secret-kid".into(), op())]).unwrap();
        assert_eq!(format!("{set:?}"), "JweOpenerSet(***)");
    }
}
