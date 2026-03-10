//! Handler 模式 - 数据存储与处理逻辑分离
//!
//! 核心设计：
//! - Handler 不依赖 Tree，只接收 Task、返回 Result
//! - NodeProcessor 负责构建 Task、分发 Handler、写回结果

use crate::{Message, NodeType};
use anyhow::Result;
use async_trait::async_trait;

/// Handler trait - 处理节点的核心接口
///
/// 职责：
/// - 接收构建好的 Task 数据
/// - 执行处理逻辑
/// - 返回 Result
///
/// 设计原则：
/// - 不依赖 ConversationTree
/// - 职责单一，易于测试
#[async_trait]
pub trait Handler: Send + Sync {
    /// 处理任务
    async fn handle(&self, task: Task) -> Result<ResultData>;
}

/// Task trait - 传递给 Handler 的数据基类
pub trait TaskBase: Send + Sync {
    /// 获取节点ID
    fn node_id(&self) -> &str;
    /// 获取节点类型
    fn node_type(&self) -> NodeType;
}

/// Result trait - Handler 返回的结果基类
pub trait ResultBase: Send + Sync {
    /// 获取节点ID
    fn node_id(&self) -> &str;
}

// ============================================================
// Task 定义
// ============================================================

/// 任务枚举 - 统一所有任务类型
#[derive(Debug, Clone)]
pub enum Task {
    /// AI 处理任务
    AI(AITask),
    /// Sampling 处理任务（MCP）
    Sampling(SamplingTask),
}

impl TaskBase for Task {
    fn node_id(&self) -> &str {
        match self {
            Task::AI(t) => t.node_id(),
            Task::Sampling(t) => t.node_id(),
        }
    }

    fn node_type(&self) -> NodeType {
        match self {
            Task::AI(t) => t.node_type(),
            Task::Sampling(t) => t.node_type(),
        }
    }
}

/// AI 处理任务
///
/// 包含处理 AI 请求所需的所有数据
#[derive(Debug, Clone)]
pub struct AITask {
    /// 节点ID
    pub node_id: String,
    /// 消息上下文
    pub messages: Vec<Message>,
    /// 模型名称 (provider/model 格式)
    pub model: String,
    /// API 端点
    pub api_base: Option<String>,
    /// 温度参数
    pub temperature: Option<f32>,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
    /// top_p 参数
    pub top_p: Option<f32>,
}

impl TaskBase for AITask {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn node_type(&self) -> NodeType {
        NodeType::User
    }
}

/// Sampling 处理任务（MCP）
///
/// MCP Server 请求采样的任务
#[derive(Debug, Clone)]
pub struct SamplingTask {
    /// 节点ID
    pub node_id: String,
    /// 消息上下文
    pub messages: Vec<Message>,
    /// 模型偏好
    pub model_preferences: Option<ModelPreferences>,
}

impl TaskBase for SamplingTask {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn node_type(&self) -> NodeType {
        NodeType::User
    }
}

/// 模型偏好（MCP Sampling）
#[derive(Debug, Clone, Default)]
pub struct ModelPreferences {
    /// 偏好的模型
    pub hints: Vec<String>,
    /// 成本优先级 (0-1)
    pub cost_priority: Option<f32>,
    /// 速度优先级 (0-1)
    pub speed_priority: Option<f32>,
    /// 智能优先级 (0-1)
    pub intelligence_priority: Option<f32>,
}

// ============================================================
// Result 定义
// ============================================================

/// 结果枚举 - 统一所有结果类型
#[derive(Debug, Clone)]
pub enum ResultData {
    /// AI 处理结果
    AI(AIResult),
    /// Sampling 处理结果
    Sampling(SamplingResult),
}

impl ResultBase for ResultData {
    fn node_id(&self) -> &str {
        match self {
            ResultData::AI(r) => r.node_id(),
            ResultData::Sampling(r) => r.node_id(),
        }
    }
}

/// AI 处理结果
#[derive(Debug, Clone)]
pub struct AIResult {
    /// 节点ID（AI 节点）
    pub node_id: String,
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub token_usage: Option<TokenUsage>,
}

impl ResultBase for AIResult {
    fn node_id(&self) -> &str {
        &self.node_id
    }
}

/// Sampling 处理结果
#[derive(Debug, Clone)]
pub struct SamplingResult {
    /// 节点ID
    pub node_id: String,
    /// 响应内容
    pub content: String,
    /// 使用的模型
    pub model: String,
}

impl ResultBase for SamplingResult {
    fn node_id(&self) -> &str {
        &self.node_id
    }
}

/// Token 使用量
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
