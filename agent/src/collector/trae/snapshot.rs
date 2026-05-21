//! Trae Git 快照解析器
//!
//! 解析 ModularData/ai-agent/snapshot/<sessionId>/ 下的 Git 仓库，
//! 通过 Tags 追踪每轮对话的代码变更。
//!
//! Tag 命名规则：
//! - chain-start-{sessionId}        — 对话链起点
//! - before-chat-turn-{turnId}      — AI 回答前的文件状态
//! - toolcall-{turnId}-{callId}     — 单次工具调用后的文件状态
//! - after-chat-turn-{turnId}       — AI 回答完成后的文件状态

use anyhow::Result;
use git2::{DiffOptions, Repository};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 快照仓库读取器
pub struct SnapshotReader {
    repo_path: PathBuf,
}

/// Tag 信息
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub kind: TagKind,
    /// 对话轮次 ID（从 tag 名提取）
    pub turn_id: Option<String>,
    /// 工具调用序号（仅 toolcall tag）
    pub call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagKind {
    ChainStart,
    BeforeChatTurn,
    AfterChatTurn,
    ToolCall,
}

/// 代码变更信息
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// 变更文件列表
    pub files: Vec<ChangedFile>,
}

/// 单个文件的变更
#[derive(Debug, Clone)]
pub struct ChangedFile {
    /// 文件路径（相对于仓库根）
    pub path: String,
    /// 新增行数
    pub lines_added: i32,
    /// 删除行数
    pub lines_removed: i32,
    /// diff 文本（skeleton）
    pub diff_text: String,
}

impl SnapshotReader {
    /// 打开快照 Git 仓库
    pub fn open(repo_path: &Path) -> Result<Self> {
        Ok(Self {
            repo_path: repo_path.to_path_buf(),
        })
    }

    /// 检查快照仓库是否存在
    pub fn exists(&self) -> bool {
        self.repo_path.exists()
    }

    /// 列出所有 tags 并解析类型
    pub fn list_tags(&self) -> Result<Vec<TagInfo>> {
        if !self.exists() {
            return Ok(vec![]);
        }

        let repo = match Repository::open(&self.repo_path) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("无法打开快照仓库 {:?}: {}", self.repo_path, e);
                return Ok(vec![]);
            }
        };

        let mut tags = Vec::new();
        for name_result in repo.tag_names(None)?.iter() {
            let name = match name_result {
                Some(n) => n.to_string(),
                None => continue,
            };

            let kind = classify_tag(&name);
            let (turn_id, call_id) = extract_tag_parts(&name, &kind);

            tags.push(TagInfo {
                name,
                kind,
                turn_id,
                call_id,
            });
        }

        Ok(tags)
    }

    /// 计算两个 tag 之间的 diff
    ///
    /// 用于 `before-chat-turn-{id}` → `after-chat-turn-{id}` 之间的代码变更
    pub fn diff_between_tags(&self, from_tag: &str, to_tag: &str) -> Result<DiffResult> {
        let repo = Repository::open(&self.repo_path)?;

        let from_tree = self.tag_to_tree(&repo, from_tag)?;
        let to_tree = self.tag_to_tree(&repo, to_tag)?;

        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(
            from_tree.as_ref(),
            to_tree.as_ref(),
            Some(&mut opts),
        )?;

        let mut files = Vec::new();
        let deltas: Vec<_> = diff.deltas().collect();
        let num_deltas = deltas.len();

        for delta_idx in 0..num_deltas {
            let delta = &deltas[delta_idx];

            // 从 delta 获取文件路径
            let path = delta
                .new_file()
                .path()
                .map(|p| normalize_path(&p.to_string_lossy()))
                .unwrap_or_default();
            if path.is_empty() {
                continue;
            }

            // 为每个文件生成 patch
            let patch = match git2::Patch::from_diff(&diff, delta_idx) {
                Ok(Some(p)) => p,
                _ => continue,
            };

            // line_stats 返回 (context_lines, additions, deletions)
            let (added, removed) = match patch.line_stats() {
                Ok((_, insertions, deletions)) => (insertions as i32, deletions as i32),
                _ => (0, 0),
            };

            // 将 patch 转为统一 diff 文本
            let diff_text = build_patch_text(&patch);

            files.push(ChangedFile {
                path,
                lines_added: added,
                lines_removed: removed,
                diff_text,
            });
        }

        Ok(DiffResult { files })
    }

    fn tag_to_tree<'a>(&self, repo: &'a Repository, tag_name: &str) -> Result<Option<git2::Tree<'a>>> {
        // 查找 tag 引用: refs/tags/<tag_name> 或直接的 tag 名
        let obj = match repo.revparse_single(tag_name) {
            Ok(o) => o,
            Err(_) => {
                let full_ref = format!("refs/tags/{}", tag_name);
                repo.revparse_single(&full_ref)?
            }
        };

        if let Some(tag) = obj.as_tag() {
            let target = tag.target()?;
            Ok(Some(target.peel_to_tree()?))
        } else if let Some(commit) = obj.as_commit() {
            Ok(Some(commit.tree()?))
        } else {
            Ok(None)
        }
    }
}

/// 将 tag 名分类
fn classify_tag(name: &str) -> TagKind {
    if name.starts_with("chain-start-") {
        TagKind::ChainStart
    } else if name.starts_with("before-chat-turn-") {
        TagKind::BeforeChatTurn
    } else if name.starts_with("after-chat-turn-") {
        TagKind::AfterChatTurn
    } else if name.starts_with("toolcall-") {
        TagKind::ToolCall
    } else {
        // 其他 tag（如 chain-end-*）忽略
        TagKind::ChainStart // 不会被使用
    }
}

/// 从 tag 名中提取 turn_id 和 call_id
fn extract_tag_parts(name: &str, kind: &TagKind) -> (Option<String>, Option<String>) {
    match kind {
        TagKind::BeforeChatTurn | TagKind::AfterChatTurn => {
            let id = name
                .strip_prefix("before-chat-turn-")
                .or_else(|| name.strip_prefix("after-chat-turn-"))
                .map(|s| s.to_string());
            (id, None)
        }
        TagKind::ToolCall => {
            let parts: Vec<&str> = name
                .strip_prefix("toolcall-")
                .unwrap_or(name)
                .splitn(2, '-')
                .collect();
            (
                parts.first().map(|s| s.to_string()),
                parts.get(1).map(|s| s.to_string()),
            )
        }
        _ => (None, None),
    }
}

/// 路径标准化：去除 base/content/ 或 disk/content/ 前缀
fn normalize_path(path: &str) -> String {
    path.strip_prefix("base/content/")
        .or_else(|| path.strip_prefix("disk/content/"))
        .unwrap_or(path)
        .to_string()
}

/// 将 git2::Patch 转为统一 diff 格式文本
fn build_patch_text(patch: &git2::Patch<'_>) -> String {
    let mut output = String::new();

    let num_hunks = patch.num_hunks();
    for hunk_idx in 0..num_hunks {
        let (hunk, _) = match patch.hunk(hunk_idx) {
            Ok(h) => h,
            Err(_) => continue,
        };

        // hunk header (e.g., "@@ -1,3 +1,4 @@")
        let header = hunk.header();
        if let Ok(header_str) = std::str::from_utf8(header) {
            output.push_str(header_str);
        }

        let num_lines = match patch.num_lines_in_hunk(hunk_idx) {
            Ok(n) => n,
            Err(_) => continue,
        };

        for line_idx in 0..num_lines {
            let line = match patch.line_in_hunk(hunk_idx, line_idx) {
                Ok(l) => l,
                Err(_) => continue,
            };

            let origin = line.origin();
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            output.push(origin);
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
        }
    }

    output
}

/// 将 tags 按轮次分组，找出 before/after 配对
///
/// 返回 Vec<(before_tag_name, after_tag_name, turn_id)>
pub fn group_turn_pairs(tags: &[TagInfo]) -> Vec<(&TagInfo, &TagInfo)> {
    let mut before_tags: HashMap<&str, &TagInfo> = HashMap::new();
    let mut after_tags: HashMap<&str, &TagInfo> = HashMap::new();

    for tag in tags {
        if let Some(ref turn_id) = tag.turn_id {
            match tag.kind {
                TagKind::BeforeChatTurn => {
                    before_tags.insert(turn_id.as_str(), tag);
                }
                TagKind::AfterChatTurn => {
                    after_tags.insert(turn_id.as_str(), tag);
                }
                _ => {}
            }
        }
    }

    let mut pairs = Vec::new();
    for (turn_id, before) in &before_tags {
        if let Some(after) = after_tags.get(turn_id) {
            pairs.push((*before, *after));
        }
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_tags() {
        assert_eq!(
            classify_tag("before-chat-turn-abc123"),
            TagKind::BeforeChatTurn
        );
        assert_eq!(
            classify_tag("after-chat-turn-abc123"),
            TagKind::AfterChatTurn
        );
        assert_eq!(classify_tag("toolcall-abc123-1"), TagKind::ToolCall);
        assert_eq!(classify_tag("chain-start-session-1"), TagKind::ChainStart);
    }

    #[test]
    fn test_extract_turn_id() {
        let (turn_id, call_id) = extract_tag_parts(
            "before-chat-turn-abc123",
            &TagKind::BeforeChatTurn,
        );
        assert_eq!(turn_id, Some("abc123".into()));
        assert_eq!(call_id, None);

        let (turn_id, call_id) =
            extract_tag_parts("toolcall-abc123-5", &TagKind::ToolCall);
        assert_eq!(turn_id, Some("abc123".into()));
        assert_eq!(call_id, Some("5".into()));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("base/content/src/main.rs"), "src/main.rs");
        assert_eq!(normalize_path("disk/content/src/lib.rs"), "src/lib.rs");
        assert_eq!(normalize_path("src/other.rs"), "src/other.rs");
    }
}
