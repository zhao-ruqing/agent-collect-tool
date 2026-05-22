// Cursor 数据解析器
// 解析 composerData JSON、aiService.generations、agent-transcripts JSONL

use anyhow::Result;
use serde::Deserialize;

// ============================================================
// composerData 数据模型
// ============================================================

/// composer.composerData 的顶层结构
#[derive(Debug, Deserialize)]
pub struct ComposerData {
    #[serde(default, rename = "allComposers")]
    pub all_composers: Vec<ComposerEntry>,
    #[serde(default, rename = "selectedComposerIds")]
    pub selected_composer_ids: Vec<String>,
    #[serde(default, rename = "lastFocusedComposerIds")]
    pub last_focused_composer_ids: Vec<String>,
    /// 数据是否已迁移到新格式（迁移后 allComposers 可能为空）
    #[serde(default, rename = "hasMigratedComposerData")]
    pub has_migrated_composer_data: bool,
}

/// 单个 Composer 会话条目
#[derive(Debug, Deserialize)]
pub struct ComposerEntry {
    #[serde(default, rename = "composerId")]
    pub composer_id: String,
    /// "agent" | "chat"
    #[serde(default, rename = "unifiedMode")]
    pub unified_mode: Option<String>,
    /// 创建时间（毫秒时间戳）
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<i64>,
    /// 最后更新时间
    #[serde(default, rename = "lastUpdatedAt")]
    pub last_updated_at: Option<i64>,
    /// 会话标题
    #[serde(default)]
    pub name: Option<String>,
    /// 会话副标题
    #[serde(default)]
    pub subtitle: Option<String>,
    /// 新增行数
    #[serde(default, rename = "totalLinesAdded")]
    pub total_lines_added: Option<i32>,
    /// 删除行数
    #[serde(default, rename = "totalLinesRemoved")]
    pub total_lines_removed: Option<i32>,
    /// 变更文件数
    #[serde(default, rename = "filesChangedCount")]
    pub files_changed_count: Option<i32>,
    /// 上下文使用百分比
    #[serde(default, rename = "contextUsagePercent")]
    pub context_usage_percent: Option<f64>,
    /// 是否已归档
    #[serde(default, rename = "isArchived")]
    pub is_archived: Option<bool>,
    /// 是否草稿
    #[serde(default, rename = "isDraft")]
    pub is_draft: Option<bool>,
    /// 是否有未读消息
    #[serde(default, rename = "hasUnreadMessages")]
    pub has_unread_messages: Option<bool>,
    /// 子 Composer 数量
    #[serde(default, rename = "numSubComposers")]
    pub num_sub_composers: Option<i32>,
    /// 引用的 plan 列表
    #[serde(default, rename = "referencedPlans")]
    pub referenced_plans: Vec<String>,
}

/// 解析 composer.composerData JSON
/// 兼容迁移前后的两种格式：
/// - 迁移前: allComposers 包含完整会话列表
/// - 迁移后 (hasMigratedComposerData=true): allComposers 为空，用 selectedComposerIds
pub fn parse_composer_data(data: &[u8]) -> Result<ComposerData> {
    if data.is_empty() {
        return Ok(ComposerData {
            all_composers: vec![],
            selected_composer_ids: vec![],
            last_focused_composer_ids: vec![],
            has_migrated_composer_data: false,
        });
    }
    let text = String::from_utf8_lossy(data);
    let mut parsed: ComposerData = serde_json::from_str(&text)?;

    // 迁移后 allComposers 为空但有 selectedComposerIds，合成 ComposerEntry
    if parsed.all_composers.is_empty() && !parsed.selected_composer_ids.is_empty() {
        let mut seen = std::collections::HashSet::new();
        for id in parsed
            .selected_composer_ids
            .iter()
            .chain(parsed.last_focused_composer_ids.iter())
        {
            if seen.insert(id) {
                parsed.all_composers.push(ComposerEntry {
                    composer_id: id.clone(),
                    unified_mode: None,
                    created_at: None,
                    last_updated_at: None,
                    name: None,
                    subtitle: None,
                    total_lines_added: None,
                    total_lines_removed: None,
                    files_changed_count: None,
                    context_usage_percent: None,
                    is_archived: None,
                    is_draft: None,
                    has_unread_messages: None,
                    num_sub_composers: None,
                    referenced_plans: vec![],
                });
            }
        }
    }

    Ok(parsed)
}

// ============================================================
// aiService.generations 数据模型
// ============================================================

#[derive(Debug, Deserialize)]
pub struct GenerationEntry {
    /// 毫秒时间戳
    #[serde(default, rename = "unixMs")]
    pub unix_ms: Option<i64>,
    /// 生成 UUID
    #[serde(default, rename = "generationUUID")]
    pub generation_uuid: Option<String>,
    /// "composer" | "apply"
    #[serde(default)]
    pub r#type: Option<String>,
    /// 描述文本
    #[serde(default, rename = "textDescription")]
    pub text_description: Option<String>,
}

/// 解析 aiService.generations JSON
pub fn parse_generations(data: &[u8]) -> Result<Vec<GenerationEntry>> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(data);
    let parsed = serde_json::from_str(&text)?;
    Ok(parsed)
}

// ============================================================
// agent-transcripts JSONL 数据模型
// ============================================================

/// 转录文件中的单行记录
#[derive(Debug, Deserialize)]
pub struct TranscriptLine {
    /// "user" | "assistant"
    pub role: String,
    pub message: TranscriptMessage,
}

#[derive(Debug, Deserialize)]
pub struct TranscriptMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool-call")]
    ToolCall {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        arguments: Option<serde_json::Value>,
        #[serde(default, rename = "toolCallId")]
        tool_call_id: Option<String>,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        #[serde(default, rename = "toolCallId")]
        tool_call_id: Option<String>,
        #[serde(default)]
        result: Option<serde_json::Value>,
    },
    /// 捕获未知内容类型
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

/// 从 transcript 行中提取纯文本（去除 XML 包裹标签）
pub fn extract_text(blocks: &[ContentBlock]) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => {
                // 去除 <user_query> 等 XML 包裹标签，保留内容
                let cleaned = strip_xml_wrapper(text);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            }
            _ => None,
        })
        .collect();
    parts.join("\n")
}

/// 统计 content blocks 中的工具调用次数
pub fn count_tool_calls(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .filter(|b| matches!(b, ContentBlock::ToolCall { .. }))
        .count()
}

/// 剥离 <user_query>...</user_query> 等 XML 包裹标签
fn strip_xml_wrapper(text: &str) -> String {
    let trimmed = text.trim();
    // 匹配 <user_query>content</user_query>
    if let Some(inner) = trimmed
        .strip_prefix("<user_query>")
        .and_then(|t| t.strip_suffix("</user_query>"))
    {
        return inner.trim().to_string();
    }
    // 匹配 <assistant_response>content</assistant_response>
    if let Some(inner) = trimmed
        .strip_prefix("<assistant_response>")
        .and_then(|t| t.strip_suffix("</assistant_response>"))
    {
        return inner.trim().to_string();
    }
    trimmed.to_string()
}

// ============================================================
// 项目路径 → .cursor/projects/ 目录名 编码
// ============================================================

/// 将项目路径编码为 Cursor 的 projects 目录名
/// "d:/Project/foo" → "d-Project-foo"
pub fn encode_project_dir(project_path: &str) -> String {
    let normalized = project_path.replace('\\', "/");
    // 先处理 Windows 盘符 (d:/ → d-)
    let step1 = if normalized.len() >= 3
        && normalized.as_bytes().get(1) == Some(&b':')
        && normalized.as_bytes().get(2) == Some(&b'/')
    {
        format!(
            "{}-{}",
            &normalized[..1],
            &normalized[3..]
        )
    } else {
        normalized.clone()
    };
    // 剩余 / 替换为 -
    step1.replace('/', "-")
}

/// 将 Cursor projects 目录名解码为项目路径
/// "d-Project-foo" → "d:/Project/foo"
pub fn decode_project_dir(dir_name: &str) -> Option<String> {
    if dir_name.is_empty() || dir_name == "empty-window" {
        return None;
    }
    let bytes = dir_name.as_bytes();
    // 盘符格式: X-...
    if bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b'-' {
        let drive = dir_name[..1].to_string();
        let rest = &dir_name[2..];
        let path = rest.replace('-', "/");
        Some(format!("{}:/{}", drive, path))
    } else {
        // Unix 路径: 第一个 / 被编码为 -
        let path = dir_name.replace('-', "/");
        Some(format!("/{}", path))
    }
}

// ============================================================
// composerHeaders（globalStorage）解析
// ============================================================

/// globalStorage 里的 composerHeaders 条目
#[derive(Debug, Deserialize)]
pub struct ComposerHeaders {
    #[serde(default, rename = "allComposers")]
    pub all_composers: Vec<ComposerHeaderEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ComposerHeaderEntry {
    #[serde(default, rename = "composerId")]
    pub composer_id: String,
    #[serde(default, rename = "workspaceIdentifier")]
    pub workspace_identifier: Option<String>,
    #[serde(default, rename = "unifiedMode")]
    pub unified_mode: Option<String>,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<i64>,
    /// 追踪的 Git 仓库
    #[serde(default, rename = "trackedGitRepos")]
    pub tracked_git_repos: Vec<String>,
}

/// 解析 composer.composerHeaders
pub fn parse_composer_headers(data: &[u8]) -> Result<Vec<ComposerHeaderEntry>> {
    if data.is_empty() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(data);
    let headers: ComposerHeaders = serde_json::from_str(&text)?;
    Ok(headers.all_composers)
}

/// 构建 composerId → 模型名 映射（从 composerHeaders 和 composerData 推断）
/// Cursor 的模型信息在 composerData 中不直接存储，暂时从 unifiedMode 推断
pub fn infer_model(mode: &Option<String>) -> Option<String> {
    match mode.as_deref() {
        Some("agent") => Some("cursor-agent".to_string()),
        Some("chat") => Some("cursor-chat".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_project_dir() {
        assert_eq!(encode_project_dir("d:/Project/foo"), "d-Project-foo");
        assert_eq!(encode_project_dir("d:\\Project\\foo"), "d-Project-foo");
        assert_eq!(
            encode_project_dir("d:/Project/asiaInfo-project"),
            "d-Project-asiaInfo-project"
        );
        assert_eq!(encode_project_dir("/home/user/project"), "-home-user-project");
    }

    #[test]
    fn test_decode_project_dir() {
        assert_eq!(
            decode_project_dir("d-Project-foo"),
            Some("d:/Project/foo".to_string())
        );
        // 含 `-` 的目录名会损失信息（- 可能是原来的分隔符也可能是名字的一部分）
        // 因此实际不依赖此函数做路径匹配，collector 通过扫描 composerId 匹配 transcript
        assert_eq!(decode_project_dir("empty-window"), None);
        assert_eq!(decode_project_dir(""), None);
    }

    #[test]
    fn test_decode_project_dir_temp() {
        let dir = "C-Users-35291-AppData-Local-Temp-be21732c-3ed5-4cb3-a69a-4755904f35a5";
        let decoded = decode_project_dir(dir);
        assert!(decoded.is_some());
        assert!(decoded.unwrap().starts_with("C:/Users/35291/AppData/Local/Temp/"));
    }

    #[test]
    fn test_strip_xml_wrapper() {
        assert_eq!(
            strip_xml_wrapper("<user_query>\nhello world\n</user_query>"),
            "hello world"
        );
        assert_eq!(
            strip_xml_wrapper("plain text without tags"),
            "plain text without tags"
        );
    }

    #[test]
    fn test_parse_composer_data_empty() {
        let result = parse_composer_data(b"").unwrap();
        assert!(result.all_composers.is_empty());
    }

    #[test]
    fn test_parse_generations_empty() {
        let result = parse_generations(b"").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_infer_model() {
        assert_eq!(
            infer_model(&Some("agent".to_string())),
            Some("cursor-agent".to_string())
        );
        assert_eq!(
            infer_model(&Some("chat".to_string())),
            Some("cursor-chat".to_string())
        );
        assert_eq!(infer_model(&None), None);
    }
}
