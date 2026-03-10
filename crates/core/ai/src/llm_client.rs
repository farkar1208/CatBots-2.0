//! LiteLLM 客户端 - 统一接口调用多厂商 LLM

use async_trait::async_trait;
use catbots_history::Message;
use catbots_profile::Profile;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LLMResponse {
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub usage: Option<TokenUsage>,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 流式响应块
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// 增量内容
    pub delta: String,
    /// 是否结束
    pub is_done: bool,
}

/// LiteLLM 客户端
/// 
/// 统一接口调用多厂商 LLM：
/// - 模型命名格式: `{provider}/{model}`
/// - 例如: openai/gpt-4o, anthropic/claude-sonnet-4-20250514, ollama/llama3.1:8b
/// - API Key 通过环境变量设置: OPENAI_API_KEY, ANTHROPIC_API_KEY 等
/// - 响应格式统一为 OpenAI 格式
pub struct LiteLLMClient {
    /// HTTP 客户端
    client: Client,
    /// 默认 API 基础 URL
    default_base_url: String,
}

impl LiteLLMClient {
    /// 创建新的 LiteLLM 客户端
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            default_base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.default_base_url = base_url.into();
        self
    }

    /// 获取 API 基础 URL
    fn get_base_url(&self, profile: &Profile) -> String {
        // 优先使用 profile 中配置的 api_base
        if let Some(ref api_base) = profile.api_base {
            return api_base.clone();
        }

        // 根据 provider 选择默认 URL
        let provider = profile.provider();
        match provider {
            "openai" => "https://api.openai.com/v1".to_string(),
            "anthropic" => "https://api.anthropic.com/v1".to_string(),
            "ollama" => "http://localhost:11434/v1".to_string(),
            _ => self.default_base_url.clone(),
        }
    }

    /// 获取 API Key
    fn get_api_key(&self, profile: &Profile) -> Option<String> {
        let provider = profile.provider();
        
        // 检查特定 provider 的环境变量
        let env_var = match provider {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "azure" => "AZURE_OPENAI_API_KEY",
            _ => &format!("{}_API_KEY", provider.to_uppercase()),
        };

        env::var(env_var).ok().or_else(|| {
            // 回退到通用的 LLM_API_KEY
            env::var("LLM_API_KEY").ok()
        })
    }

    /// 完成请求（非流式）
    /// 
    /// # 参数
    /// - `messages`: 消息列表
    /// - `profile`: 配置档案
    pub async fn complete(
        &self,
        messages: Vec<Message>,
        profile: &Profile,
    ) -> Result<LLMResponse, anyhow::Error> {
        let base_url = self.get_base_url(profile);
        let api_key = self.get_api_key(profile);

        // 构建请求体
        let request_body = ChatCompletionRequest {
            model: profile.model_name().to_string(),
            messages: messages
                .into_iter()
                .map(|m| ChatMessage {
                    role: match m.role {
                        catbots_history::MessageRole::System => "system".to_string(),
                        catbots_history::MessageRole::User => "user".to_string(),
                        catbots_history::MessageRole::Assistant => "assistant".to_string(),
                        catbots_history::MessageRole::Tool => "tool".to_string(),
                    },
                    content: m.content,
                })
                .collect(),
            temperature: profile.parameters.temperature,
            max_tokens: profile.parameters.max_tokens,
            top_p: profile.parameters.top_p,
            stream: false,
        };

        let url = format!("{}/chat/completions", base_url);
        
        tracing::debug!(
            url = %url,
            model = %profile.model_name(),
            "发送 LLM 请求"
        );

        let mut request = self.client.post(&url).json(&request_body);

        // 添加认证头
        if let Some(key) = api_key {
            request = request.bearer_auth(&key);
        }

        // 对于 Anthropic，需要额外的头
        if profile.provider() == "anthropic" {
            request = request
                .header("anthropic-version", "2023-06-01")
                .header("x-api-key", env::var("ANTHROPIC_API_KEY").unwrap_or_default());
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM API error: {} - {}", status, body));
        }

        let completion: ChatCompletionResponse = response.json().await?;

        let choice = completion
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            model: completion.model,
            usage: completion.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    /// 流式完成请求
    /// 
    /// # 参数
    /// - `messages`: 消息列表
    /// - `profile`: 配置档案
    pub async fn stream_complete(
        &self,
        _messages: Vec<Message>,
        _profile: &Profile,
    ) -> Result<(), anyhow::Error> {
        // TODO: 实现流式 API 调用
        Err(anyhow::anyhow!("Stream completion not implemented yet"))
    }
}

impl Default for LiteLLMClient {
    fn default() -> Self {
        Self::new()
    }
}

/// LLM 客户端 trait（保留兼容性）
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// 完成请求（非流式）
    async fn complete(&self, messages: Vec<Message>) -> Result<LLMResponse, anyhow::Error>;
    
    /// 流式完成请求
    async fn stream_complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<(), anyhow::Error>;
}

// ============ OpenAI API 数据结构 ============

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: String,
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_extraction() {
        let profile = Profile::new("test", "Test", "openai/gpt-4o");
        assert_eq!(profile.provider(), "openai");
        assert_eq!(profile.model_name(), "gpt-4o");
    }

    #[test]
    fn test_anthropic_provider() {
        let profile = Profile::new("test", "Test", "anthropic/claude-sonnet-4-20250514");
        assert_eq!(profile.provider(), "anthropic");
        assert_eq!(profile.model_name(), "claude-sonnet-4-20250514");
    }
}