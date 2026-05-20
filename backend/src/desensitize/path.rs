use regex::Regex;
use std::sync::LazyLock;

/// 文件路径脱敏：将绝对路径中的用户名替换为 `<user>`
///
/// Windows: C:\Users\john\Project\... → <user>\Project\...
/// Unix:    /home/john/project/...    → /home/<user>/project/...
static RE_WINDOWS_USER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([A-Z]:\\Users\\)[^\\]+").unwrap());

static RE_UNIX_USER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(/home/)[^/]+").unwrap());

pub fn desensitize_path(path: &str) -> String {
    let path = RE_WINDOWS_USER.replace_all(path, "${1}<user>");
    let path = RE_UNIX_USER.replace_all(&path, "${1}<user>");
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_path() {
        let result = desensitize_path(r"C:\Users\john\Project\agent\src\main.rs");
        assert_eq!(result, r"C:\Users\<user>\Project\agent\src\main.rs");
    }

    #[test]
    fn test_unix_path() {
        let result = desensitize_path("/home/alice/projects/agent/src/main.rs");
        assert_eq!(result, "/home/<user>/projects/agent/src/main.rs");
    }

    #[test]
    fn test_no_user_path() {
        let result = desensitize_path(r"D:\Projects\shared\lib.rs");
        assert_eq!(result, r"D:\Projects\shared\lib.rs");
    }
}
