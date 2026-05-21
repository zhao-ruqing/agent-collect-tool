-- 002_add_content_and_api_key.sql
-- 为 messages 表添加 content 原文列，为 agents 表添加 api_key 列

USE agent_collect_tool;

-- messages 表：添加原始内容列（用户查看用）
ALTER TABLE messages ADD COLUMN IF NOT EXISTS content TEXT AFTER content_hash;

-- agents 表：添加 API 密钥列（HMAC 签名验证用）
ALTER TABLE agents ADD COLUMN IF NOT EXISTS api_key VARCHAR(128) AFTER hostname_hash;
