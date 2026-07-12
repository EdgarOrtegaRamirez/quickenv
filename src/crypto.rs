use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;

/// Generate a 256-bit key from a passphrase
pub fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .expect("key derivation should succeed");
    key
}

/// Encrypt plaintext using AES-256-GCM
/// Returns: salt (16 bytes) + nonce (12 bytes) + ciphertext
pub fn encrypt(plaintext: &str, passphrase: &str) -> Vec<u8> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption should succeed");

    let mut result = Vec::with_capacity(salt.len() + nonce.len() + ciphertext.len());
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    result
}

/// Decrypt ciphertext using AES-256-GCM
/// Expects: salt (16 bytes) + nonce (12 bytes) + ciphertext
pub fn decrypt(data: &[u8], passphrase: &str) -> anyhow::Result<String> {
    if data.len() < 28 {
        anyhow::bail!("Invalid encrypted data: too short");
    }
    let (salt, rest) = data.split_at(16);
    let (nonce_bytes, ciphertext) = rest.split_at(12);

    let key = derive_key(passphrase, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).expect("valid key");
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed: wrong passphrase or corrupted data"))?;

    Ok(String::from_utf8(plaintext)?)
}

/// Encrypt a file in-place (write to .env.encrypted)
pub fn encrypt_file(input_path: &std::path::Path, passphrase: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(input_path)?;
    let encrypted = encrypt(&content, passphrase);

    let output_path = input_path.with_extension("env.encrypted");
    std::fs::write(&output_path, &encrypted)?;
    eprintln!(
        "Encrypted: {} → {}",
        input_path.display(),
        output_path.display()
    );
    Ok(())
}

/// Decrypt a file
pub fn decrypt_file(input_path: &std::path::Path, passphrase: &str) -> anyhow::Result<String> {
    let data = std::fs::read(input_path)?;
    decrypt(&data, passphrase)
}

/// Generate a random master key as hex
pub fn generate_master_key() -> String {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    hex::encode(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = "DATABASE_URL=postgres://localhost:5432/mydb\nAPI_KEY=secret123";
        let passphrase = "test-master-key-12345";
        let encrypted = encrypt(plaintext, passphrase);
        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let plaintext = "SECRET=value";
        let encrypted = encrypt(plaintext, "correct-passphrase");
        let result = decrypt(&encrypted, "wrong-passphrase");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_master_key() {
        let key1 = generate_master_key();
        let key2 = generate_master_key();
        assert_eq!(key1.len(), 64); // 32 bytes = 64 hex chars
        assert_ne!(key1, key2); // random, should differ
    }
}
