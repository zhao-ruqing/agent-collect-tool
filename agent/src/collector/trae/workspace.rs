use anyhow::Result;
use std::path::{Path, PathBuf};

/// 单个 Trae 工作区信息
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// 工作区哈希（workspaceStorage 下的目录名）
    pub hash: String,
    /// 项目路径（从 workspace.json 解析）
    pub project_path: String,
    /// state.vscdb 文件路径
    pub vscdb_path: PathBuf,
}

/// 遍历 workspaceStorage 目录，发现所有活跃工作区
///
/// 数据目录结构：
/// ```text
/// %APPDATA%/Trae/User/workspaceStorage/
/// ├── <hash1>/
/// │   ├── state.vscdb        ← 核心 K-V 数据库
/// │   ├── state.vscdb.backup
/// │   └── workspace.json     ← 项目路径映射
/// └── <hash2>/
///     └── ...
/// ```
pub fn discover_workspaces(trae_user_dir: &Path) -> Result<Vec<WorkspaceInfo>> {
    let ws_dir = trae_user_dir.join("workspaceStorage");
    if !ws_dir.exists() || !ws_dir.is_dir() {
        log::debug!("Trae workspaceStorage 目录不存在: {:?}", ws_dir);
        return Ok(vec![]);
    }

    let mut workspaces = Vec::new();

    for entry in std::fs::read_dir(&ws_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::debug!("读取 workspaceStorage 条目失败: {}", e);
                continue;
            }
        };

        let hash_dir = entry.path();
        if !hash_dir.is_dir() {
            continue;
        }

        let vscdb_path = hash_dir.join("state.vscdb");
        if !vscdb_path.exists() {
            continue;
        }

        let hash = hash_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let project_path = read_workspace_path(&hash_dir.join("workspace.json"))
            .unwrap_or_default();

        workspaces.push(WorkspaceInfo {
            hash,
            project_path,
            vscdb_path,
        });
    }

    log::debug!("发现 {} 个 Trae 工作区", workspaces.len());
    Ok(workspaces)
}

/// 从 workspace.json 读取项目文件夹路径
///
/// workspace.json 格式:
/// ```json
/// { "folder": "file:///d%3A/Project/agent-collect-tool" }
/// ```
fn read_workspace_path(json_path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(json_path)?;
    let v: serde_json::Value = serde_json::from_str(&content)?;

    // 优先读取 folder 字段
    if let Some(uri) = v.get("folder").and_then(|f| f.as_str()) {
        return Ok(file_uri_to_path(uri));
    }

    // 兼容 folders 数组格式
    if let Some(folders) = v.get("folders").and_then(|f| f.as_array()) {
        if let Some(first) = folders.first() {
            if let Some(uri) = first.get("uri").and_then(|f| f.as_str()) {
                return Ok(file_uri_to_path(uri));
            }
        }
    }

    Ok(String::new())
}

/// 判断是否 Windows 盘符路径（如 "d%3A/..." 或 "C:/..."）
fn is_windows_drive_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // X: 格式（如 "d:/..."）
    if bytes[0].is_ascii_alphabetic() && bytes.len() >= 2 && bytes[1] == b':' {
        return true;
    }
    // X%3A 格式（URL 编码的冒号）
    if bytes[0].is_ascii_alphabetic()
        && bytes.len() >= 4
        && (&bytes[1..4] == b"%3A" || &bytes[1..4] == b"%3a")
    {
        return true;
    }
    false
}

/// 将 file:// URI 转换为文件系统路径
///
/// 示例: "file:///d%3A/Project/agent-collect-tool" → "d:/Project/agent-collect-tool"
///       "file:///home/user/project" → "/home/user/project"
fn file_uri_to_path(uri: &str) -> String {
    // 1. 去掉协议前缀
    // file:/// 的第三个 / 对于 Unix 是路径一部分，对于 Windows 盘符路径则不是
    let after_proto = if let Some(rest) = uri.strip_prefix("file:///") {
        if is_windows_drive_path(rest) || rest.starts_with('/') {
            rest.to_string()
        } else {
            format!("/{}", rest)
        }
    } else if let Some(rest) = uri.strip_prefix("file://") {
        rest.to_string()
    } else {
        uri.to_string()
    };

    // 2. URL 解码
    let decoded = match urlencoding::decode(&after_proto) {
        Ok(cow) => cow.into_owned(),
        Err(_) => after_proto,
    };

    // 3. Windows 路径规范化为正斜杠
    decoded.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_uri_decode() {
        let result = file_uri_to_path("file:///d%3A/Project/test");
        assert_eq!(result, "d:/Project/test");

        let result = file_uri_to_path("file:///home/user/project");
        assert_eq!(result, "/home/user/project");
    }
}
