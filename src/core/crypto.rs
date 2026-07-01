use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::Path;

const NONCE_SIZE: usize = 12;

/// Deriva uma chave AES-256 a partir de uma senha.
pub fn derive_key(password: &str) -> Key<Aes256Gcm> {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    *Key::<Aes256Gcm>::from_slice(&result)
}

/// Criptografa dados com AES-256-GCM.
pub fn encrypt(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let key = derive_key(password);
    let cipher = Aes256Gcm::new(&key);

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Erro ao criptografar: {}", e))?;

    // Formato: nonce (12 bytes) + ciphertext
    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Descriptografa dados com AES-256-GCM.
pub fn decrypt(encrypted_data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if encrypted_data.len() < NONCE_SIZE {
        return Err("Dados criptografados inválidos".to_string());
    }

    let key = derive_key(password);
    let cipher = Aes256Gcm::new(&key);

    let nonce = Nonce::from_slice(&encrypted_data[..NONCE_SIZE]);
    let ciphertext = &encrypted_data[NONCE_SIZE..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Erro ao descriptografar: {}", e))
}

/// Criptografa um arquivo e salva com extensão .f1enc.
pub fn encrypt_file(path: &Path, password: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("Erro ao ler arquivo: {}", e))?;

    let encrypted = encrypt(&data, password)?;

    let mut encrypted_path = path.to_path_buf();
    let new_name = format!(
        "{}.f1enc",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    encrypted_path.set_file_name(new_name);

    std::fs::write(&encrypted_path, encrypted)
        .map_err(|e| format!("Erro ao salvar arquivo criptografado: {}", e))?;

    // Remove o arquivo original
    std::fs::remove_file(path).map_err(|e| format!("Erro ao remover original: {}", e))?;

    Ok(())
}

/// Descriptografa um arquivo .f1enc.
pub fn decrypt_file(path: &Path, password: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("Erro ao ler arquivo: {}", e))?;

    let decrypted = decrypt(&data, password)?;

    let mut decrypted_path = path.to_path_buf();
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let new_name = name.trim_end_matches(".f1enc");
    decrypted_path.set_file_name(new_name);

    std::fs::write(&decrypted_path, decrypted)
        .map_err(|e| format!("Erro ao salvar arquivo descriptografado: {}", e))?;

    std::fs::remove_file(path)
        .map_err(|e| format!("Erro ao remover arquivo criptografado: {}", e))?;

    Ok(())
}
