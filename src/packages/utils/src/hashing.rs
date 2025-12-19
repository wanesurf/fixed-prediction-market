use sha2::{Digest, Sha256};

/// A simple hashing function that hashes a list of strings using SHA-256.
pub fn hash_data(data: Vec<&str>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    
    data.iter().for_each(|d| {
        hasher.update(d.as_bytes());
    });

    hasher.finalize().to_vec()
}
