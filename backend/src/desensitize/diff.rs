use regex::Regex;
use std::sync::LazyLock;

/// 字符串字面量匹配：双引号或单引号内的内容
static RE_STRING_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""[^"]*""#).unwrap());

/// 数字字面量匹配：独立的数字（含小数、负数）
static RE_NUMBER_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d+(?:\.\d+)?\b").unwrap());

/// Diff 骨架化：替换字符串/数字字面量为占位符，保留代码结构
pub fn skeletonize_diff(diff: &str) -> String {
    diff.lines()
        .map(|line| {
            if line.starts_with('+') || line.starts_with('-') {
                let prefix = &line[..1];
                let content = &line[1..];
                let content = RE_STRING_LITERAL.replace_all(content, "<str>");
                let content = RE_NUMBER_LITERAL.replace_all(&content, "<num>");
                format!("{}{}", prefix, content)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 截断过长的 diff（防止单次上报数据过大）
#[allow(dead_code)]
pub fn truncate_diff(diff: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = diff.lines().take(max_lines).collect();
    let mut result = lines.join("\n");
    if diff.lines().count() > max_lines {
        result.push_str(&format!(
            "\n... 省略 {} 行",
            diff.lines().count() - max_lines
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal_replacement() {
        let line = r#"+    let name = "hello world";"#;
        let result = skeletonize_diff(line);
        assert_eq!(result, r#"+    let name = <str>;"#);
    }

    #[test]
    fn test_number_replacement() {
        let line = "-    let x = 42;";
        let result = skeletonize_diff(line);
        assert_eq!(result, "-    let x = <num>;");
    }

    #[test]
    fn test_float_replacement() {
        let line = "+    let y = 3.14;";
        let result = skeletonize_diff(line);
        assert_eq!(result, "+    let y = <num>;");
    }

    #[test]
    fn test_context_line_unchanged() {
        let line = "     let z = 100;";
        let result = skeletonize_diff(line);
        assert_eq!(result, "     let z = 100;");
    }

    #[test]
    fn test_truncate() {
        let diff = "line1\nline2\nline3\nline4\nline5";
        let result = truncate_diff(diff, 3);
        assert!(result.starts_with("line1\nline2\nline3\n..."));
    }
}
