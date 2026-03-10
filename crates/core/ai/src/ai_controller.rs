//! AI 控制器 - 协调 LLM 调用与对话树交互

use crate::{LiteLLMClient, TokenUsage};
use catbots_history::ConversationTree;
use catbots_profile::Profile;
use std::sync::{Arc, Mutex};

/// AI 控制器
/// 
/// 核心职责：
/// - 从 ConversationTree 获取上下文
/// - 调用 LLM API
/// - 将结果写回 ConversationTree
pub struct AIController {
    /// LiteLLM 客户端
    client: LiteLLMClient,
    /// 对话树（共享引用）
    tree: Arc<Mutex<ConversationTree>>,
}

impl AIController {
    /// 创建新的 AI 控制器
    pub fn new(tree: Arc<Mutex<ConversationTree>>) -> Self {
        Self {
            client: LiteLLMClient::new(),
            tree,
        }
    }

    /// 发送请求（非流式）
    /// 
    /// # 参数
    /// - `node_id`: 当前节点ID（用户消息节点）
    /// - `profile`: 配置档案
    /// 
    /// # 返回
    /// - AI 节点ID
    pub async fn send(
        &self,
        node_id: &str,
        profile: &Profile,
    ) -> Result<AIResponse, anyhow::Error> {
        // 1. 从 tree 获取上下文
        let messages = {
            let tree = self.tree.lock().map_err(|e| anyhow::anyhow!("Tree lock error: {}", e))?;
            tree.get_context(node_id)
        };

        tracing::debug!(
            node_id = %node_id,
            message_count = messages.len(),
            "获取上下文完成"
        );

        // 2. 调用 LLM
        let response = self.client.complete(messages, profile).await?;

        tracing::info!(
            model = %response.model,
            tokens = ?response.usage,
            "LLM 响应完成"
        );

        // 3. 将结果写入 tree
        let ai_node_id = {
            let mut tree = self.tree.lock().map_err(|e| anyhow::anyhow!("Tree lock error: {}", e))?;
            tree.add_ai_node(node_id, response.content.clone(), response.model.clone())
        };

        Ok(AIResponse {
            node_id: ai_node_id,
            content: response.content,
            model: response.model,
            usage: response.usage,
        })
    }

    /// 发送请求（流式）
    /// 
    /// # 参数
    /// - `node_id`: 当前节点ID
    /// - `profile`: 配置档案
    /// 
    /// # 返回
    /// - 流式响应生成器
    pub async fn stream(
        &self,
        _node_id: &str,
        _profile: &Profile,
    ) -> Result<(), anyhow::Error> {
        // TODO: 实现流式发送
        // 1. 从 tree 获取上下文
        // 2. 调用 LLM 流式 API: client.stream_complete(messages, profile)
        // 3. 实时返回增量内容
        // 4. 流结束后将完整结果写入 tree
        Err(anyhow::anyhow!("流式响应尚未实现"))
    }

    /// 获取对话树的克隆引用（用于外部访问）
    pub fn tree(&self) -> Arc<Mutex<ConversationTree>> {
        self.tree.clone()
    }
}

/// AI 响应结果
#[derive(Debug, Clone)]
pub struct AIResponse {
    /// AI 节点ID
    pub node_id: String,
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub usage: Option<TokenUsage>,
}