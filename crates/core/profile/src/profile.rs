//! 配置档案

use serde::{Deserialize, Serialize};
use crate::ModelParameters;

/// 配置档案
/// 
/// 存储模型配置和参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// 唯一标识
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 模型名称 (LiteLLM 格式: provider/model)
    /// 例如: openai/gpt-4o, anthropic/claude-sonnet-4-20250514, ollama/llama3.1:8b
    pub model: String,
    /// API 端点 (可选，用于本地模型等)
    pub api_base: Option<String>,
    /// 模型参数
    pub parameters: ModelParameters,
    /// 是否为默认配置
    pub is_default: bool,
}

impl Profile {
    /// 创建新的 Profile
    pub fn new(id: impl Into<String>, name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            api_base: None,
            parameters: ModelParameters::default(),
            is_default: false,
        }
    }

    /// 设置为默认配置
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// 设置 API 端点
    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = Some(api_base.into());
        self
    }

    /// 设置模型参数
    pub fn with_parameters(mut self, parameters: ModelParameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// 获取 provider 名称
    pub fn provider(&self) -> &str {
        self.model.split('/').next().unwrap_or("unknown")
    }

    /// 获取模型名称（不含 provider 前缀）
    pub fn model_name(&self) -> &str {
        self.model.split('/').nth(1).unwrap_or(&self.model)
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new("default", "默认配置", "openai/gpt-4o").as_default()
    }
}
