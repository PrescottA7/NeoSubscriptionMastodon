use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use std::path::Path;

/// Symmetric key used to protect credentials at rest. It lives next to the
/// database, so it mainly protects database-only leaks such as stray backups.
#[derive(Clone)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    /// Loads the key from `path`, generating it on first run.
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if let Ok(bytes) = std::fs::read(path) {
            let key: [u8; 32] = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("{} must contain exactly 32 bytes", path.display()))?;
            return Ok(Self(key));
        }

        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, key)?;
        restrict_permissions(path)?;

        Ok(Self(key))
    }

    /// Encrypts a UTF-8 value, returning a self-describing base64 blob.
    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<String> {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.0)
            .map_err(|_| anyhow::anyhow!("invalid encryption key length"))?;

        let mut nonce_bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| anyhow::anyhow!("failed to encrypt value"))?;

        let mut blob = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);

        Ok(base64::engine::general_purpose::STANDARD.encode(blob))
    }

    /// Reverses [`EncryptionKey::encrypt`].
    pub fn decrypt(&self, encoded: &str) -> anyhow::Result<String> {
        if encoded.is_empty() {
            return Ok(String::new());
        }

        let blob = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| anyhow::anyhow!("stored value is not valid base64"))?;

        if blob.len() <= 24 {
            return Err(anyhow::anyhow!("stored value is truncated"));
        }

        let (nonce_bytes, ciphertext) = blob.split_at(24);
        let cipher = XChaCha20Poly1305::new_from_slice(&self.0)
            .map_err(|_| anyhow::anyhow!("invalid encryption key length"))?;

        let nonce: [u8; 24] = nonce_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("stored value is truncated"))?;

        let plaintext = cipher
            .decrypt(&XNonce::from(nonce), ciphertext)
            .map_err(|_| anyhow::anyhow!("failed to decrypt value"))?;

        String::from_utf8(plaintext).map_err(|_| anyhow::anyhow!("decrypted value is not UTF-8"))
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_key() -> EncryptionKey {
        EncryptionKey([7u8; 32])
    }

    #[test]
    fn round_trips_a_secret() {
        let key = test_key();
        let encrypted = key.encrypt("hunter2").expect("encrypt");
        assert_ne!(encrypted, "hunter2");
        assert_eq!(key.decrypt(&encrypted).expect("decrypt"), "hunter2");
    }

    #[test]
    fn same_plaintext_yields_different_ciphertext() {
        let key = test_key();
        let first = key.encrypt("same").expect("encrypt");
        let second = key.encrypt("same").expect("encrypt");
        assert_ne!(first, second);
    }

    #[test]
    fn empty_value_round_trips() {
        let key = test_key();
        assert_eq!(key.decrypt("").expect("decrypt"), "");
    }

    #[test]
    fn tampered_value_fails() {
        let key = test_key();
        let mut encrypted = key.encrypt("hunter2").expect("encrypt");
        encrypted.pop();
        encrypted.push('A');
        assert!(key.decrypt(&encrypted).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let encrypted = test_key().encrypt("hunter2").expect("encrypt");
        assert!(EncryptionKey([9u8; 32]).decrypt(&encrypted).is_err());
    }

    #[test]
    fn key_file_is_created_once() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("encryption.key");

        let first = EncryptionKey::load_or_create(&path).expect("create");
        let second = EncryptionKey::load_or_create(&path).expect("load");

        assert_eq!(first.0, second.0);
        assert_eq!(std::fs::read(&path).expect("read").len(), 32);
    }
}
