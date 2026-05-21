//! Trae vscdb JSON 数据解析器
//!
//! 将 vscdb 中的原始 JSON BLOB 解析为标准化数据结构，
//! 供 TraeCollector 组装为 RawEvent。

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

// ============================================================
// 数据模型
// ============================================================

/// 会话条目（来自 memento/icube-ai-agent-storage）
#[derive(Debug, Deserialize)]
pub struct SessionEntry {
    /// 会话 ID（如 "session_1" 或 UUID）
    #[serde(default)]
    #[serde(alias = "sessionId")]
    pub id: Option<String>,
    /// 会话标题
    #[serde(default)]
    pub title: Option<String>,
    /// 是否为当前活跃会话
    #[serde(default, rename = "isCurrent")]
    pub is_current: bool,
    /// 创建时间戳（毫秒）
    #[serde(default)]
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    /// 最后活跃时间
    #[serde(default)]
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<i64>,
}

/// 用户输入条目（来自 icube-ai-agent-storage-input-history）
#[derive(Debug, Deserialize)]
pub struct UserInputEntry {
    /// 所属 sessionId（当前版本 Trae 不存储此字段，由 collector 按序号关联）
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
    /// 输入文本内容（Trae 中字段名为 inputText）
    #[serde(default)]
    #[serde(alias = "inputText")]
    pub text: Option<String>,
    /// 解析后的查询（可能包含文件引用和纯文本片段）
    #[serde(default, rename = "parsedQuery")]
    pub parsed_query: Option<Vec<serde_json::Value>>,
    /// 引用的文件路径列表
    #[serde(default)]
    pub files: Option<Vec<FileRef>>,
    /// 多媒体附件
    #[serde(default, rename = "multiMedia")]
    pub multi_media: Option<Vec<serde_json::Value>>,
    /// 时间戳（毫秒）
    #[serde(default)]
    pub timestamp: Option<i64>,
}

/// 文件引用
#[derive(Debug, Deserialize)]
pub struct FileRef {
    /// 文件路径
    #[serde(default)]
    pub file: Option<String>,
    /// 文件起始行
    #[serde(default)]
    pub start: Option<u32>,
    /// 文件结束行
    #[serde(default)]
    pub end: Option<u32>,
}

// ============================================================
// 解析函数
// ============================================================

/// 解析会话列表 → Vec<SessionEntry>
pub fn parse_session_list(data: &[u8]) -> Result<Vec<SessionEntry>> {
    if data.is_empty() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(data);

    // 尝试解析为数组
    if let Ok(entries) = serde_json::from_str::<Vec<SessionEntry>>(&text) {
        return Ok(entries);
    }

    // 兼容：可能是嵌套的对象 {"list": [...]} 或 {"sessions": [...]} 或 {"value": [...]}
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = v.get("list")
            .or_else(|| v.get("sessions"))
            .or_else(|| v.get("value"))
            .or_else(|| v.get("data"))
            .and_then(|x| x.as_array())
        {
            let entries: Vec<SessionEntry> = arr
                .iter()
                .filter_map(|e| serde_json::from_value(e.clone()).ok())
                .collect();
            return Ok(entries);
        }
    }

    Ok(vec![])
}

/// 解析会话→Agent 类型映射（icube_session_agent_map）
///
/// 返回 HashMap<sessionId, agentType>
/// agentType 示例: "builder", "dev_agent", "solo_agent"
pub fn parse_agent_map(data: &[u8]) -> Result<HashMap<String, String>> {
    if data.is_empty() {
        return Ok(HashMap::new());
    }

    let text = String::from_utf8_lossy(&data);
    let map: HashMap<String, String> = serde_json::from_str(&text).unwrap_or_default();
    Ok(map)
}

/// 解析会话→模型名映射（{userId}_ai-chat:sessionRelation:modelMap）
///
/// 返回 HashMap<sessionId, modelName>
/// modelName 示例: "1_-_gemini-3-flash-premium"
pub fn parse_model_map(data: &[u8]) -> Result<HashMap<String, String>> {
    if data.is_empty() {
        return Ok(HashMap::new());
    }

    let text = String::from_utf8_lossy(&data);

    // 格式可能是: {"sessionId": {"agentType": "1_-_gemini-3-flash-premium"}}
    // 或: {"sessionId": {"model": "1_-_claude-sonnet-4-6"}}
    // 或: {"sessionId": "1_-_gemini-3-flash-premium"}
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        let mut map = HashMap::new();
        if let Some(obj) = v.as_object() {
            for (k, val) in obj {
                let model = if let Some(s) = val.as_str() {
                    s.to_string()
                } else if let Some(inner) = val.as_object() {
                    // 优先尝试 "model" 键，否则取第一个值
                    inner
                        .get("model")
                        .or_else(|| inner.values().next())
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    continue;
                };
                if !model.is_empty() {
                    map.insert(k.clone(), model);
                }
            }
        }
        return Ok(map);
    }

    Ok(HashMap::new())
}

/// 解析用户输入历史列表
pub fn parse_input_history(data: &[u8]) -> Result<Vec<UserInputEntry>> {
    if data.is_empty() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&data);

    if let Ok(entries) = serde_json::from_str::<Vec<UserInputEntry>>(&text) {
        return Ok(entries);
    }

    // 兼容嵌套格式
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = v.get("entries")
            .or_else(|| v.get("value"))
            .or_else(|| v.get("data"))
            .and_then(|x| x.as_array())
        {
            let entries: Vec<UserInputEntry> = arr
                .iter()
                .filter_map(|e| serde_json::from_value(e.clone()).ok())
                .collect();
            return Ok(entries);
        }
    }

    Ok(vec![])
}

/// 解析 Agent 配置（currentAgentData_{userId}）
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent 名称（如 "Builder"）
    pub name: Option<String>,
    /// Agent 类型（如 "builder"）
    pub agent_type: Option<String>,
    /// 模型名称
    pub model: Option<String>,
    /// 启用的工具列表
    pub tools: Vec<String>,
}

pub fn parse_agent_config(data: &[u8]) -> Result<AgentConfig> {
    if data.is_empty() {
        return Ok(AgentConfig {
            name: None,
            agent_type: None,
            model: None,
            tools: vec![],
        });
    }

    let text = String::from_utf8_lossy(&data);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

    // 工具列表：Trae 使用 "built_in_tool_list" 字段，项为 {"value": "toolName"} 格式
    let tools = v
        .get("built_in_tool_list")
        .or_else(|| v.get("tools"))
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| t.get("value").and_then(|x| x.as_str()).map(|s| s.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(AgentConfig {
        name: v.get("name").and_then(|x| x.as_str()).map(|s| s.to_string()),
        agent_type: v
            .get("type")
            .or_else(|| v.get("unique_name"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        // Trae AgentData 中通常没有 model 字段，模型信息在 model_map 中
        model: v.get("model").and_then(|x| x.as_str()).map(|s| s.to_string()),
        tools,
    })
}

/// 从 ChatStore 中估算对话轮次数
///
/// ChatStore 存储对话 UI 状态，包含每条消息的高度信息
pub fn estimate_turn_count(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    let text = String::from_utf8_lossy(&data);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        // ChatStore 中可能有 messages 数组，其长度 = 对话轮次
        if let Some(messages) = v.get("messages").and_then(|x| x.as_array()) {
            return messages.len();
        }
        // 或者可能是 chat 对象的 messages
        if let Some(chat) = v.get("chat").or_else(|| v.get("chats")).and_then(|x| x.as_array()) {
            return chat.len();
        }
    }

    0
}

/// 从 session entry 提取 session ID
pub fn extract_session_ids(entries: &[SessionEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|e| e.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_session_list_array() {
        let json = br#"[{"id":"session_1","title":"Hello","createdAt":1700000000000}]"#;
        let entries = parse_session_list(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.as_deref(), Some("session_1"));
    }

    #[test]
    fn test_parse_session_list_nested() {
        let json = br#"{"sessions":[{"id":"session_2","title":"World"}]}"#;
        let entries = parse_session_list(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.as_deref(), Some("session_2"));
    }

    #[test]
    fn test_parse_agent_map() {
        let json = br#"{"session_1":"builder","session_2":"dev_agent"}"#;
        let map = parse_agent_map(json).unwrap();
        assert_eq!(map.get("session_1").unwrap(), "builder");
        assert_eq!(map.get("session_2").unwrap(), "dev_agent");
    }

    #[test]
    fn test_parse_model_map_simple() {
        let json = br#"{"session_1":"1_-_gemini-3-flash-premium"}"#;
        let map = parse_model_map(json).unwrap();
        assert_eq!(map.get("session_1").unwrap(), "1_-_gemini-3-flash-premium");
    }

    #[test]
    fn test_parse_model_map_nested() {
        let json = br#"{"session_1":{"model":"1_-_claude-sonnet-4-6"}}"#;
        let map = parse_model_map(json).unwrap();
        assert_eq!(map.get("session_1").unwrap(), "1_-_claude-sonnet-4-6");
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_session_list(b"").unwrap().is_empty());
        assert!(parse_agent_map(b"").unwrap().is_empty());
        assert!(parse_model_map(b"").unwrap().is_empty());
        assert!(parse_input_history(b"").unwrap().is_empty());
    }

    #[test]
    fn test_parse_input_history() {
        // 使用 String + as_bytes 避免 raw byte literal 中的非 ASCII 字符
        let json = r#"[{"sessionId":"s1","text":"帮我写个函数","timestamp":1700000000000}]"#;
        let entries = parse_input_history(json.as_bytes()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text.as_deref(), Some("\u{5e2e}\u{6211}\u{5199}\u{4e2a}\u{51fd}\u{6570}"));
    }

    #[test]
    fn test_parse_agent_config() {
        let json = br#"{"name":"Builder","type":"builder","model":"gemini-3-flash","tools":["read_file","write_file"]}"#;
        let config = parse_agent_config(json).unwrap();
        assert_eq!(config.name.as_deref(), Some("Builder"));
        assert_eq!(config.agent_type.as_deref(), Some("builder"));
        assert_eq!(config.model.as_deref(), Some("gemini-3-flash"));
        assert_eq!(config.tools.len(), 2);
    }

    #[test]
    fn test_extract_session_ids() {
        let entries = vec![
            SessionEntry { id: Some("s1".into()), title: None, created_at: None, updated_at: None, is_current: false },
            SessionEntry { id: Some("s2".into()), title: None, created_at: None, updated_at: None, is_current: false },
            SessionEntry { id: None, title: None, created_at: None, updated_at: None, is_current: false },
        ];
        let ids = extract_session_ids(&entries);
        assert_eq!(ids, vec!["s1", "s2"]);
    }
}
