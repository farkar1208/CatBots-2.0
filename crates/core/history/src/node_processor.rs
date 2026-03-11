//! 节点处理器 - 处理协调中心
//!
//! 核心职责：
//! - 从 Tree 获取节点和上下文
//! - 从 ProfileManager 获取配置
//! - 构建 Task 对象
//! - 分发到对应的 Handler
//! - 接收 Handler 返回的 Result
//! - 写回 Tree

use crate::{
    AITask, AIResult, ConversationTree, Handler, Message, NodeEnum, NodeType, ResultData,
    SamplingResult, SamplingTask, Task, TaskBase,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 节点处理器
///
/// 作为处理协调中心，负责：
/// 1. 从 Tree 获取数据
/// 2. 构建 Task
/// 3. 分发到 Handler
/// 4. 写回结果
pub struct NodeProcessor {
    /// 对话树
    tree: Arc<Mutex<ConversationTree>>,
    /// 处理器映射 (NodeType -> Handler)
    handlers: HashMap<NodeType, Arc<dyn Handler>>,
    /// 默认模型
    default_model: String,
    /// 默认 API 端点
    default_api_base: Option<String>,
    /// 默认温度
    default_temperature: Option<f32>,
    /// 默认最大 token
    default_max_tokens: Option<u32>,
}

impl NodeProcessor {
    /// 创建新的节点处理器
    pub fn new(tree: Arc<Mutex<ConversationTree>>) -> Self {
        Self {
            tree,
            handlers: HashMap::new(),
            default_model: "openai/gpt-4o".to_string(),
            default_api_base: None,
            default_temperature: Some(0.7),
            default_max_tokens: Some(4096),
        }
    }

    /// 设置默认模型
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 设置默认 API 端点
    pub fn with_default_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.default_api_base = Some(api_base.into());
        self
    }

    /// 设置默认温度
    pub fn with_default_temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = Some(temperature);
        self
    }

    /// 设置默认最大 token
    pub fn with_default_max_tokens(mut self, max_tokens: u32) -> Self {
        self.default_max_tokens = Some(max_tokens);
        self
    }

    /// 注册处理器
    pub fn register_handler(&mut self, node_type: NodeType, handler: Arc<dyn Handler>) {
        self.handlers.insert(node_type, handler);
    }

    /// 请求处理节点
    ///
    /// # 参数
    /// - `node_id`: 要处理的节点ID
    ///
    /// # 返回
    /// 处理结果
    pub async fn request_process(&self, node_id: &str) -> Result<ResultData> {
        // 1. 获取节点信息
        let node = {
            let tree = self.tree.lock().await;
            tree.get_node(node_id)
                .ok_or_else(|| anyhow!("节点不存在: {}", node_id))?
        };

        // 2. 获取上下文
        let messages = {
            let tree = self.tree.lock().await;
            tree.get_context(node_id)
        };

        // 3. 构建任务
        let task = self.build_task(&node, messages)?;

        // 4. 分发到处理器
        let handler = self
            .handlers
            .get(&task.node_type())
            .ok_or_else(|| anyhow!("未注册处理器: {:?}", task.node_type()))?;

        let result = handler.handle(task).await?;

        // 5. 写回结果
        self.write_result(&result).await?;

        Ok(result)
    }

    /// 构建 Task
    fn build_task(&self, node: &NodeEnum, messages: Vec<Message>) -> Result<Task> {
        match node {
            NodeEnum::User(_) => {
                // User 节点 -> AI 任务
                Ok(Task::AI(AITask {
                    node_id: node.id().to_string(),
                    messages,
                    model: self.default_model.clone(),
                    api_base: self.default_api_base.clone(),
                    temperature: self.default_temperature,
                    max_tokens: self.default_max_tokens,
                    top_p: None,
                }))
            }
            _ => Err(anyhow!("不支持的节点类型: {:?}", node.node_type())),
        }
    }

    /// 写回结果到 Tree
    async fn write_result(&self, result: &ResultData) -> Result<()> {
        let tree = self.tree.lock().await;

        match result {
            ResultData::AI(ai_result) => {
                // AI 结果：AIController 已经直接添加到 Tree 中，无需再次写入
                tracing::info!(
                    ai_node_id = %ai_result.node_id,
                    model = %ai_result.model,
                    "AI节点已由AIController直接创建"
                );
            }
            ResultData::Sampling(sampling_result) => {
                // Sampling 结果更新节点
                tracing::info!(
                    node_id = %sampling_result.node_id,
                    "Sampling 处理完成"
                );
            }
        }

        Ok(())
    }

    /// 更新处理配置
    pub fn update_config(
        &mut self,
        model: String,
        api_base: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) {
        self.default_model = model;
        self.default_api_base = api_base;
        self.default_temperature = temperature;
        self.default_max_tokens = max_tokens;
    }

    /// 获取对话树引用
    pub fn tree(&self) -> Arc<Mutex<ConversationTree>> {
        self.tree.clone()
    }
}
