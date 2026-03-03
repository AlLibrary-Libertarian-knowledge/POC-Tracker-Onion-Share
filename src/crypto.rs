use anyhow::Context;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use blake3::Hasher;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use uuid::Uuid;

pub type FileKey = [u8; 32];

pub fn random_key() -> FileKey {
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    k
}

pub fn key_to_b64url(key: &FileKey) -> String {
    URL_SAFE_NO_PAD.encode(key)
}

pub fn key_from_b64url(s: &str) -> anyhow::Result<FileKey> {
    let raw = URL_SAFE_NO_PAD.decode(s).context("invalid base64url key")?;
    anyhow::ensure!(raw.len() == 32, "key must be 32 bytes");
    let mut k = [0u8; 32];
    k.copy_from_slice(&raw);
    Ok(k)
}

/// Deterministic per-chunk nonce derived from (key, chunk_index).
/// XChaCha requires 24-byte nonce; we take the first 24 bytes of a keyed BLAKE3 hash.
pub fn nonce_for_chunk(key: &FileKey, chunk_index: u64) -> XNonce {
    let mut hasher = Hasher::new_keyed(key);
    hasher.update(&chunk_index.to_le_bytes());
    let out = hasher.finalize();
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&out.as_bytes()[0..24]);
    XNonce::from_slice(&nonce).to_owned()
}

fn aad(file_id: Uuid, chunk_index: u64) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[..16].copy_from_slice(file_id.as_bytes());
    out[16..].copy_from_slice(&chunk_index.to_le_bytes());
    out
}

pub fn encrypt_chunk(key: &FileKey, file_id: Uuid, chunk_index: u64, plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32 bytes");
    let nonce = nonce_for_chunk(key, chunk_index);
    let aad = aad(file_id, chunk_index);

    let ct = cipher
        .encrypt(&nonce, Payload { msg: plaintext, aad: &aad })
        .map_err(|_| anyhow::anyhow!("encrypt failed"))?;
    Ok(ct)
}

pub fn decrypt_chunk(key: &FileKey, file_id: Uuid, chunk_index: u64, ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).expect("32 bytes");
    let nonce = nonce_for_chunk(key, chunk_index);
    let aad = aad(file_id, chunk_index);

    let pt = cipher
        .decrypt(&nonce, Payload { msg: ciphertext, aad: &aad })
        .map_err(|_| anyhow::anyhow!("decrypt failed (wrong key or corrupted data)"))?;
    Ok(pt)
}
