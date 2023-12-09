//! Key exchange.

use super::GUID;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha1::{Digest, Sha1};

/// Generate a new `sec-websocket-key`.
#[inline]
pub fn new_sec_key() -> [u8; 24] {
    let input: [u8; 16] = rand::random();
    let mut output = [0_u8; 24];
    Engine::encode_slice(&STANDARD, input, &mut output).unwrap();
    output
}

/// Derive `sec-websocket-accept` from `sec-websocket-key`.
#[inline]
pub fn derive_accept_key(sec_key: &[u8]) -> [u8; 28] {
    let mut sha1 = Sha1::default();
    sha1.update(sec_key);
    sha1.update(GUID);
    let input = sha1.finalize();
    let mut output = [0_u8; 28];
    Engine::encode_slice(&STANDARD, input, &mut output).unwrap();
    output
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generate_sec_key() {
        for _ in 0..=1024 {
            // should not panic
            new_sec_key();
        }
    }

    #[test]
    fn derive_sec_key() {
        assert_eq!(
            &derive_accept_key(b"dGhlIHNhbXBsZSBub25jZQ=="),
            b"s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }
}
