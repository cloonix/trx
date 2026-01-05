//! ID generation for trx issues
//!
//! Uses hash-based IDs for conflict-free merges across git clones.
//! Format: trx-xxxx (4 lowercase alphanumeric chars)

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate a unique issue ID
///
/// Uses UUID + timestamp hash, encoded as base32 lowercase.
/// Format: trx-xxxx where xxxx is 4 alphanumeric chars.
pub fn generate_id(prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    let mut hasher = Sha256::new();
    hasher.update(uuid.as_bytes());
    hasher.update(timestamp.to_le_bytes());

    let hash = hasher.finalize();

    // Take first 4 bytes, encode as base32 lowercase, take first 4 chars
    let encoded = base32::encode(base32::Alphabet::Crockford, &hash[..4])
        .to_lowercase()
        .chars()
        .take(4)
        .collect::<String>();

    format!("{}-{}", prefix, encoded)
}

/// Generate a child ID from parent
///
/// Format: parent-id.N where N is the child number
pub fn generate_child_id(parent_id: &str, child_num: u32) -> String {
    format!("{}.{}", parent_id, child_num)
}

/// Parse an issue ID to extract prefix and hash
pub fn parse_id(id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    if parts.len() == 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

/// Check if ID is a child (contains dots)
pub fn is_child_id(id: &str) -> bool {
    id.contains('.')
}

/// Get parent ID from child ID
pub fn get_parent_id(id: &str) -> Option<&str> {
    if let Some(pos) = id.rfind('.') {
        Some(&id[..pos])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id = generate_id("trx");
        assert!(id.starts_with("trx-"));
        assert_eq!(id.len(), 8); // trx- + 4 chars
    }

    #[test]
    fn test_generate_child_id() {
        let child = generate_child_id("trx-abc1", 1);
        assert_eq!(child, "trx-abc1.1");

        let grandchild = generate_child_id("trx-abc1.1", 2);
        assert_eq!(grandchild, "trx-abc1.1.2");
    }

    #[test]
    fn test_get_parent_id() {
        assert_eq!(get_parent_id("trx-abc1.1"), Some("trx-abc1"));
        assert_eq!(get_parent_id("trx-abc1.1.2"), Some("trx-abc1.1"));
        assert_eq!(get_parent_id("trx-abc1"), None);
    }
}
