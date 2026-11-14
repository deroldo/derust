//! The only module allowed to import `josekit`. Swapping the JOSE backend
//! (plan B) must stay confined to this file.
#![allow(dead_code)]

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
