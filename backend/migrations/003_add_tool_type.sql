-- 003_add_tool_type.sql
-- 为 sessions 表添加 tool_type 列，区分不同 AI 工具（claude-code、trae 等）

USE agent_collect_tool;

ALTER TABLE sessions ADD COLUMN tool_type VARCHAR(32) AFTER git_branch;
