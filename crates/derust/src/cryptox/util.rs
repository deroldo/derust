//! Small internal helpers (hex, base64url, random bytes).
#![allow(dead_code)]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Maximum accepted size (bytes) of a JWT before any parsing.
pub(crate) const MAX_TOKEN_LEN: usize = 16 * 1024;
/// Maximum accepted size (bytes) of a compact JWE.
pub(crate) const MAX_JWE_LEN: usize = 64 * 1024;

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub(crate) fn hex_decode(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    fn nibble(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    bytes
        .chunks(2)
        .map(|p| Some((nibble(p[0])? << 4) | nibble(p[1])?))
        .collect()
}

pub(crate) fn b64url_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes base64url without padding; padded or standard-alphabet input is rejected.
pub(crate) fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(input).ok()
}

pub(crate) fn random_bytes(n: usize) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let mut buf = vec![0u8; n];
    openssl::rand::rand_bytes(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let data = [0u8, 1, 0xab, 0xff];
        let enc = hex_encode(&data);
        assert_eq!(enc, "0001abff");
        assert_eq!(hex_decode(&enc).unwrap(), data);
        assert_eq!(hex_decode("ABFF").unwrap(), vec![0xab, 0xff]);
    }

    #[test]
    fn hex_rejects_invalid() {
        assert!(hex_decode("abc").is_none());
        assert!(hex_decode("zz").is_none());
        assert_eq!(hex_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn b64url_round_trip_without_padding() {
        let data = [251u8, 255, 254, 1];
        let enc = b64url_encode(&data);
        assert!(!enc.contains('=') && !enc.contains('+') && !enc.contains('/'));
        assert_eq!(b64url_decode(&enc).unwrap(), data);
    }

    #[test]
    fn b64url_rejects_invalid() {
        assert!(b64url_decode("a=").is_none());
        assert!(b64url_decode("a+b/").is_none());
        assert!(b64url_decode("!!!!").is_none());
    }

    #[test]
    fn random_bytes_has_requested_length_and_varies() {
        let a = random_bytes(16).unwrap();
        let b = random_bytes(16).unwrap();
        assert_eq!(a.len(), 16);
        assert_ne!(a, b);
    }
}
