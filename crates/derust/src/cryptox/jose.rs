//! The only module allowed to import `josekit`. Swapping the JOSE backend
//! (plan B) must stay confined to this file.

use crate::cryptox::keys::{SigningKey, VerifyingKey};
use crate::cryptox::util::{b64url_decode, b64url_encode};
use josekit::jws::{JwsSigner, JwsVerifier, ES256};
use zeroize::Zeroizing;

/// Length in bytes of a raw ES256 signature (`r || s`).
pub(crate) const ES256_SIG_LEN: usize = 64;

/// Opaque failure of a JOSE primitive; carries no detail on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JoseFailure;

/// Signs `header_json`/`payload` as a compact JWS with ES256. The header is used verbatim.
pub(crate) fn jws_sign_es256(
    key: &SigningKey,
    header_json: &[u8],
    payload: &[u8],
) -> Result<String, JoseFailure> {
    let pem = Zeroizing::new(
        key.pkey()
            .private_key_to_pem_pkcs8()
            .map_err(|_| JoseFailure)?,
    );
    let signer = ES256
        .signer_from_pem(pem.as_slice())
        .map_err(|_| JoseFailure)?;
    let signing_input = format!("{}.{}", b64url_encode(header_json), b64url_encode(payload));
    let signature = signer
        .sign(signing_input.as_bytes())
        .map_err(|_| JoseFailure)?;
    if signature.len() != ES256_SIG_LEN {
        return Err(JoseFailure);
    }
    Ok(format!("{signing_input}.{}", b64url_encode(&signature)))
}

/// Verifies the ES256 signature of a compact JWS and returns the (now authentic) payload bytes.
/// Does not interpret the header; callers validate it before calling.
pub(crate) fn jws_verify_es256(key: &VerifyingKey, token: &str) -> Result<Vec<u8>, JoseFailure> {
    let mut parts = token.split('.');
    let (header, payload, signature) =
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(h), Some(p), Some(s), None) => (h, p, s),
            _ => return Err(JoseFailure),
        };
    let signature = b64url_decode(signature).ok_or(JoseFailure)?;
    if signature.len() != ES256_SIG_LEN {
        return Err(JoseFailure);
    }
    let payload_bytes = b64url_decode(payload).ok_or(JoseFailure)?;
    let pem = key.pkey().public_key_to_pem().map_err(|_| JoseFailure)?;
    let verifier = ES256.verifier_from_pem(&pem).map_err(|_| JoseFailure)?;
    let signing_input = format!("{header}.{payload}");
    verifier
        .verify(signing_input.as_bytes(), &signature)
        .map_err(|_| JoseFailure)?;
    Ok(payload_bytes)
}

/// Test helper: checks that `josekit` accepts the JWK as a public EC P-256 key.
#[cfg(test)]
pub(crate) fn jwk_is_valid_p256(jwk: &serde_json::Value) -> bool {
    match josekit::jwk::Jwk::from_bytes(jwk.to_string()) {
        Ok(parsed) => {
            parsed.key_type() == "EC"
                && parsed.curve() == Some("P-256")
                && parsed.parameter("d").is_none()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod test {
    use josekit::jwe::{JweHeader, ECDH_ES};
    use josekit::jws::{JwsHeader, ES256};
    use openssl::ec::{EcGroup, EcKey};
    use openssl::nid::Nid;
    use openssl::pkey::PKey;

    fn keypair() -> (Vec<u8>, Vec<u8>) {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        (
            key.private_key_to_pem_pkcs8().unwrap(),
            key.public_key_to_pem().unwrap(),
        )
    }

    #[test]
    fn josekit_es256_sign_and_verify_smoke() {
        let (private, public) = keypair();
        let signer = ES256.signer_from_pem(&private).unwrap();
        let verifier = ES256.verifier_from_pem(&public).unwrap();
        let mut header = JwsHeader::new();
        header.set_token_type("JWT");
        let token = josekit::jws::serialize_compact(b"{\"a\":1}", &header, &signer).unwrap();
        let (payload, _) = josekit::jws::deserialize_compact(&token, &verifier).unwrap();
        assert_eq!(payload, b"{\"a\":1}");
    }

    #[test]
    fn josekit_ecdh_es_a256gcm_round_trip_smoke() {
        let (private, public) = keypair();
        let encrypter = ECDH_ES.encrypter_from_pem(&public).unwrap();
        let decrypter = ECDH_ES.decrypter_from_pem(&private).unwrap();
        let mut header = JweHeader::new();
        header.set_content_encryption("A256GCM");
        let jwe = josekit::jwe::serialize_compact(b"secret", &header, &encrypter).unwrap();
        assert_eq!(jwe.split('.').count(), 5);
        let (payload, _) = josekit::jwe::deserialize_compact(&jwe, &decrypter).unwrap();
        assert_eq!(payload, b"secret");
    }
}
