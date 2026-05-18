use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn generate_agent_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn hash_hostname(hostname: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    hasher.update(salt.as_bytes());
    format!("{:x}", hasher.finalize())
}
