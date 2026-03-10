//! AI 控制器 - 实现 Handler trait

use crate::{LiteLLMClient, LLMResponse};
use async_trait::async_trait;
use catbots_history::{AIResult, AITask, Handler, Message, ResultData, Task, TokenUsage};

/// AI 控制器
///
/// 核心职责：
/// - 实现 Handler trait
/// - 接收 AITask，调用 LLM，返回 AIResult
/// - 不依赖 ConversationTree
pub struct AIController {
    /// LiteLLM 客户端
    client: LiteLLMClient,
}

impl AIController {
    /// 创建新的 AI 控制器
    pub fn new() -> Self {
        Self {
            client: LiteLLMClient::new(),
        }
    }

    /// 使用自定义基础 URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.client = self.client.with_base_url(base_url);
        self
    }

    /// 处理 AI 任务（内部方法）
    async fn handle_ai_task(&self, task: AITask) -> Result<AIResult, anyhow::Error> {
        tracing::debug!(
            node_id = %task.node_id,
            model = %task.model,
            message_count = task.messages.len(),
            "处理 AI 任务"
        );

        // 调用 LLM
        let response = self.complete(task.messages, &task.model, &task.api_base, task.temperature, task.max_tokens).await?;

        tracing::info!(
            node_id = %task.node_id,
            model = %response.model,
            tokens = ?response.usage,
            "LLM 响应完成"
        );

        Ok(AIResult {
            node_id: task.node_id,
            content: response.content,
            model: response.model,
            token_usage: response.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    /// 调用 LLM API
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        api_base: &Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<LLMResponse, anyhow::Error> {
        // 构建临时 Profile 用于调用 LiteLLMClient
        let profile = catbots_profile::Profile::new("temp", "temp", model)
            .with_parameters(catbots_profile::ModelParameters {
                temperature,
                max_tokens,
                ..Default::default()
            });
        
        let profile = if let Some(base) = api_base {
            profile.with_api_base(base)
        } else {
            profile
        };

        self.client.complete(messages, &profile).await
    }

    /// 发送请求（兼容旧接口，用于过渡）
    ///
    /// # 参数
    /// - `messages`: 消息上下文
    /// - `model`: 模型名称
    /// - `api_base`: API 端点
    /// - `temperature`: 温度参数
    /// - `max_tokens`: 最大 token
    ///
    /// # 返回
    /// AI 响应
    pub async fn send(
        &self,
        messages: Vec<Message>,
        model: String,
        api_base: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<AIResponse, anyhow::Error> {
        let response = self.complete(messages, &model, &api_base, temperature, max_tokens).await?;

        Ok(AIResponse {
            content: response.content,
            model: response.model,
            usage: response.usage.map(|u| crate::TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

impl Default for AIController {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for AIController {
    async fn handle(&self, task: Task) -> Result<ResultData, anyhow::Error> {
        match task {
            Task::AI(ai_task) => {
                let result = self.handle_ai_task(ai_task).await?;
                Ok(ResultData::AI(result))
            }
            Task::Sampling(sampling_task) => {
                // Sampling 任务也用 AI 处理
                let ai_task = AITask {
                    node_id: sampling_task.node_id,
                    messages: sampling_task.messages,
                    model: "openai/gpt-4o".to_string(), // 默认模型
                    api_base: None,
                    temperature: None,
                    max_tokens: None,
                    top_p: None,
                };
                let result = self.handle_ai_task(ai_task).await?;
                Ok(ResultData::AI(result))
            }
        }
    }
}

/// AI 响应结果（兼容旧接口）
#[derive(Debug, Clone)]
pub struct AIResponse {
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub usage: Option<crate::TokenUsage>,
}
