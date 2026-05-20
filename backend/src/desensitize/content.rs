use sha2::{Digest, Sha256};

/// 对对话内容做 SHA256 哈希 + 截断摘要
///
/// 原始内容不入库，只存储哈希值用于去重，摘要用于管理后台展示。
/// 返回 (content_hash, summary)
pub fn desensitize_content(content: &str) -> (String, String) {
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let summary = truncate_summary(content, 200);

    (hash, summary)
}

/// 截取前 N 个字符作为摘要，按 UTF-8 字符边界截断
fn truncate_summary(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        content.to_string()
    } else {
        let summary: String = content.chars().take(max_chars).collect();
        format!("{}...", summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let (hash1, _) = desensitize_content("hello");
        let (hash2, _) = desensitize_content("hello");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_difference() {
        let (hash1, _) = desensitize_content("hello");
        let (hash2, _) = desensitize_content("world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_truncation() {
        let long_text = "a".repeat(300);
        let (_, summary) = desensitize_content(&long_text);
        assert!(summary.len() <= 203); // 200 chars + "..."
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_no_truncation() {
        let short_text = "hello world";
        let (_, summary) = desensitize_content(short_text);
        assert_eq!(summary, short_text);
    }
}
