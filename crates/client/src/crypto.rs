// use aes_gcm::{Aes256Gcm, Key, Nonce};
// use aes_gcm::aead::{Aead, KeyInit};
// use sha2::{Sha256, Digest};
// use rand::RngCore;

// pub fn derive_key(secret: &str) -> [u8; 32] {
//     let mut hasher = Sha256::new();
//     hasher.update(secret.as_bytes());
//     hasher.finalize().into()
// }

// pub fn encrypt(secret: &str, plaintext: &str) -> Vec<u8> {
//     let key_bytes = derive_key(secret);
//     let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
//     let cipher = Aes256Gcm::new(key);

//     let mut nonce_bytes = [0u8; 12];
//     rand::thread_rng().fill_bytes(&mut nonce_bytes);
//     let nonce = Nonce::from_slice(&nonce_bytes);

//     let mut ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
//         .expect("encryption failed");

//     let mut result = nonce_bytes.to_vec();
//     result.append(&mut ciphertext);

//     result
// }

// pub fn decrypt(secret: &str, data: &[u8]) -> String {
//     let key_bytes = derive_key(secret);
//     let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
//     let cipher = Aes256Gcm::new(key);

//     let (nonce_bytes, ciphertext) = data.split_at(12);
//     let nonce = Nonce::from_slice(nonce_bytes);

//     let plaintext = cipher.decrypt(nonce, ciphertext)
//         .expect("decryption failed");

//     String::from_utf8(plaintext).unwrap()
// }