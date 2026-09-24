//! Decrypting zip entries.
//!
//! Zip carries two incompatible schemes:
//!
//! * **WinZip AES** (method 99 plus extra field 0x9901) — the modern one:
//!   PBKDF2-HMAC-SHA1, AES-CTR, and HMAC-SHA1 for integrity;
//! * **ZipCrypto** — the legacy scheme from the 1990s. It is cryptographically
//!   weak (falls to a known-plaintext attack), but archives using it still
//!   turn up, so it has to be read. We only ever read it, never create it.
//!
//! Both produce a compressed stream, which the ordinary read paths take from
//! there. Encryption comes off exactly one layer below decompression.

use aes::cipher::{KeyIvInit, StreamCipher};
use anyhow::{bail, Result};
use hmac::digest::KeyInit;
use hmac::{Hmac, Mac};
use sha1::Sha1;

use zipmount_i18n::t;

use crate::secret::{PasswordError, Secret};
use crate::zipfmt::{AesInfo, RawEntry};

/// PBKDF2 iteration count, fixed by the WinZip AES specification.
const AES_PBKDF2_ROUNDS: u32 = 1000;
/// The truncated HMAC-SHA1 at the end of the entry.
const AES_AUTH_CODE_LEN: usize = 10;
/// Two bytes for a quick password check before decrypting the whole body.
const AES_PV_LEN: usize = 2;
const ZIPCRYPTO_HEADER_LEN: usize = 12;

/// Takes the encryption off an entry's raw bytes and returns the compressed
/// stream.
pub fn decrypt_entry(raw: &[u8], entry: &RawEntry, password: Option<&Secret>) -> Result<Vec<u8>> {
    let password = password.ok_or_else(PasswordError::missing)?;

    match entry.aes {
        Some(info) => decrypt_aes(raw, info, password),
        None => decrypt_zipcrypto(raw, entry, password),
    }
}

fn decrypt_aes(raw: &[u8], info: AesInfo, password: &Secret) -> Result<Vec<u8>> {
    let (salt_len, key_len) = match info.strength {
        1 => (8usize, 16usize),
        2 => (12, 24),
        3 => (16, 32),
        other => bail!(t!("core-aes-unknown-strength", strength = other)),
    };

    let overhead = salt_len + AES_PV_LEN + AES_AUTH_CODE_LEN;
    if raw.len() < overhead {
        bail!(t!("core-encrypted-too-short"));
    }

    let salt = &raw[..salt_len];
    let pv = &raw[salt_len..salt_len + AES_PV_LEN];
    let ciphertext = &raw[salt_len + AES_PV_LEN..raw.len() - AES_AUTH_CODE_LEN];
    let auth_code = &raw[raw.len() - AES_AUTH_CODE_LEN..];

    // Three things are derived from the password at once: the encryption
    // key, the HMAC key, and two bytes for a quick password check.
    let mut derived = vec![0u8; key_len * 2 + AES_PV_LEN];
    pbkdf2::pbkdf2_hmac::<Sha1>(password.bytes(), salt, AES_PBKDF2_ROUNDS, &mut derived);

    let enc_key = &derived[..key_len];
    let auth_key = &derived[key_len..key_len * 2];
    let expected_pv = &derived[key_len * 2..];

    if expected_pv != pv {
        return Err(PasswordError::wrong().into());
    }

    // Check the authentication code before decrypting: if it does not match,
    // "decrypted" bytes must not be handed out — nothing vouches for them.
    let mut mac = Hmac::<Sha1>::new_from_slice(auth_key).expect("HMAC accepts a key of any length");
    mac.update(ciphertext);
    let tag = mac.finalize().into_bytes();
    if tag[..AES_AUTH_CODE_LEN] != *auth_code {
        bail!(t!("core-aes-auth-failed"));
    }

    let mut out = ciphertext.to_vec();
    apply_aes_ctr(info.strength, enc_key, &mut out)?;
    Ok(out)
}

/// AES-CTR the WinZip way: the counter is a 128-bit little-endian number
/// starting at one. That is exactly where it differs from the usual NIST
/// big-endian counter, and it is an easy place to go wrong.
fn apply_aes_ctr(strength: u8, key: &[u8], data: &mut [u8]) -> Result<()> {
    let mut iv = [0u8; 16];
    iv[0] = 1;

    match strength {
        1 => ctr::Ctr128LE::<aes::Aes128>::new_from_slices(key, &iv)
            .map_err(|_| anyhow::anyhow!("bad AES-128 key length"))?
            .apply_keystream(data),
        2 => ctr::Ctr128LE::<aes::Aes192>::new_from_slices(key, &iv)
            .map_err(|_| anyhow::anyhow!("bad AES-192 key length"))?
            .apply_keystream(data),
        3 => ctr::Ctr128LE::<aes::Aes256>::new_from_slices(key, &iv)
            .map_err(|_| anyhow::anyhow!("bad AES-256 key length"))?
            .apply_keystream(data),
        other => bail!(t!("core-aes-unknown-strength", strength = other)),
    }
    Ok(())
}

fn decrypt_zipcrypto(raw: &[u8], entry: &RawEntry, password: &Secret) -> Result<Vec<u8>> {
    if raw.len() < ZIPCRYPTO_HEADER_LEN {
        bail!(t!("core-encrypted-too-short"));
    }

    let mut keys = ZipCryptoKeys::new(password.bytes());

    let mut header = [0u8; ZIPCRYPTO_HEADER_LEN];
    for (i, &byte) in raw[..ZIPCRYPTO_HEADER_LEN].iter().enumerate() {
        header[i] = keys.decrypt(byte);
    }

    // The header's last byte is the check byte. If the entry was written as a
    // stream (it has a data descriptor), the archiver puts the high byte of
    // the time there, otherwise the high byte of the CRC32.
    let has_descriptor = entry.flags & (1 << 3) != 0;
    let expected = if has_descriptor {
        (entry.dos_time >> 8) as u8
    } else {
        (entry.crc32 >> 24) as u8
    };
    if header[ZIPCRYPTO_HEADER_LEN - 1] != expected {
        return Err(PasswordError::wrong().into());
    }

    let mut out = Vec::with_capacity(raw.len() - ZIPCRYPTO_HEADER_LEN);
    for &byte in &raw[ZIPCRYPTO_HEADER_LEN..] {
        out.push(keys.decrypt(byte));
    }
    Ok(out)
}

/// Three 32-bit keys that evolve with every byte processed.
struct ZipCryptoKeys {
    k0: u32,
    k1: u32,
    k2: u32,
}

impl ZipCryptoKeys {
    fn new(password: &[u8]) -> Self {
        let mut keys = Self {
            k0: 0x1234_5678,
            k1: 0x2345_6789,
            k2: 0x3456_7890,
        };
        for &byte in password {
            keys.update(byte);
        }
        keys
    }

    fn update(&mut self, byte: u8) {
        self.k0 = crc32_byte(self.k0, byte);
        self.k1 = self
            .k1
            .wrapping_add(self.k0 & 0xff)
            .wrapping_mul(134_775_813)
            .wrapping_add(1);
        self.k2 = crc32_byte(self.k2, (self.k1 >> 24) as u8);
    }

    fn keystream_byte(&self) -> u8 {
        let temp = (self.k2 | 2) & 0xffff;
        ((temp.wrapping_mul(temp ^ 1) >> 8) & 0xff) as u8
    }

    fn decrypt(&mut self, cipher: u8) -> u8 {
        let plain = cipher ^ self.keystream_byte();
        self.update(plain);
        plain
    }
}

/// A byte-wise CRC32 update without the inversions — the form ZipCrypto
/// uses; not the same as an entry's checksum.
fn crc32_byte(crc: u32, byte: u8) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *slot = c;
        }
        t
    });
    (crc >> 8) ^ table[((crc ^ byte as u32) & 0xff) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_byte_matches_reference_stream() {
        // Running through the byte-wise update must match the ordinary CRC32
        // once its customary inversions are applied.
        let data = b"123456789";
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc = crc32_byte(crc, b);
        }
        assert_eq!(crc ^ 0xFFFF_FFFF, 0xCBF4_3926);
    }

    #[test]
    fn zipcrypto_keys_are_deterministic() {
        let a = ZipCryptoKeys::new(b"secret");
        let b = ZipCryptoKeys::new(b"secret");
        assert_eq!((a.k0, a.k1, a.k2), (b.k0, b.k1, b.k2));
    }

    #[test]
    fn zipcrypto_different_passwords_diverge() {
        let a = ZipCryptoKeys::new(b"secret");
        let b = ZipCryptoKeys::new(b"secreT");
        assert_ne!((a.k0, a.k1, a.k2), (b.k0, b.k1, b.k2));
    }

    #[test]
    fn zipcrypto_roundtrip_with_matching_keystream() {
        // Encrypt "by hand" with the same keystream and make sure decryption
        // gives back the original.
        let plain = b"hello, encrypted world";
        let mut enc_keys = ZipCryptoKeys::new(b"pw");
        let cipher: Vec<u8> = plain
            .iter()
            .map(|&p| {
                let c = p ^ enc_keys.keystream_byte();
                enc_keys.update(p);
                c
            })
            .collect();

        let mut dec_keys = ZipCryptoKeys::new(b"pw");
        let got: Vec<u8> = cipher.iter().map(|&c| dec_keys.decrypt(c)).collect();
        assert_eq!(got, plain);
    }

    #[test]
    fn aes_ctr_is_its_own_inverse() {
        let key = [7u8; 32];
        let original = b"AES-CTR is a stream cipher".to_vec();
        let mut buf = original.clone();
        apply_aes_ctr(3, &key, &mut buf).unwrap();
        assert_ne!(buf, original, "the ciphertext must differ");
        apply_aes_ctr(3, &key, &mut buf).unwrap();
        assert_eq!(buf, original);
    }

    #[test]
    fn aes_rejects_unknown_strength() {
        let mut buf = [0u8; 16];
        assert!(apply_aes_ctr(9, &[0u8; 32], &mut buf).is_err());
    }
}
