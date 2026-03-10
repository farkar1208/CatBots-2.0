//! 节点数据结构

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// 根节点
    Root,
    /// 用户消息节点
    User,
    /// AI 响应节点
    AI,
    /// 工具调用节点
    Tool,
}

/// 节点基础 trait
pub trait Node: Send + Sync {
    /// 获取节点ID
    fn id(&self) -> &str;
    
    /// 获取父节点ID
    fn parent_id(&self) -> Option<&str>;
    
    /// 获取子节点列表
    fn children(&self) -> &[String];
    
    /// 添加子节点
    fn add_child(&mut self, child_id: String);
    
    /// 获取时间戳
    fn timestamp(&self) -> &DateTime<Utc>;
    
    /// 获取节点类型
    fn node_type(&self) -> NodeType;
}

/// 根节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootNode {
    pub id: String,
    pub children: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl RootNode {
    pub fn new() -> Self {
        Self {
            id: "root".to_string(),
            children: Vec::new(),
            timestamp: Utc::now(),
        }
    }
}

impl Default for RootNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for RootNode {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn parent_id(&self) -> Option<&str> {
        None
    }
    
    fn children(&self) -> &[String] {
        &self.children
    }
    
    fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }
    
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::Root
    }
}

/// 用户消息节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNode {
    pub id: String,
    pub parent_id: String,
    pub children: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    /// 附件列表（文件路径等）
    pub attachments: Vec<String>,
}

impl UserNode {
    pub fn new(parent_id: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            children: Vec::new(),
            timestamp: Utc::now(),
            content,
            attachments: Vec::new(),
        }
    }
}

impl Node for UserNode {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn parent_id(&self) -> Option<&str> {
        Some(&self.parent_id)
    }
    
    fn children(&self) -> &[String] {
        &self.children
    }
    
    fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }
    
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::User
    }
}

/// AI 响应节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AINode {
    pub id: String,
    pub parent_id: String,
    pub children: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    /// 使用的模型名称
    pub model: String,
    /// Token 使用量
    pub token_usage: Option<TokenUsage>,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl AINode {
    pub fn new(parent_id: String, content: String, model: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            children: Vec::new(),
            timestamp: Utc::now(),
            content,
            model,
            token_usage: None,
        }
    }
}

impl Node for AINode {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn parent_id(&self) -> Option<&str> {
        Some(&self.parent_id)
    }
    
    fn children(&self) -> &[String] {
        &self.children
    }
    
    fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }
    
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::AI
    }
}

/// 工具调用节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNode {
    pub id: String,
    pub parent_id: String,
    pub children: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

impl ToolNode {
    pub fn new(parent_id: String, tool_name: String, input: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            children: Vec::new(),
            timestamp: Utc::now(),
            tool_name,
            input,
            output: serde_json::Value::Null,
        }
    }
}

impl Node for ToolNode {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn parent_id(&self) -> Option<&str> {
        Some(&self.parent_id)
    }
    
    fn children(&self) -> &[String] {
        &self.children
    }
    
    fn add_child(&mut self, child_id: String) {
        self.children.push(child_id);
    }
    
    fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
    
    fn node_type(&self) -> NodeType {
        NodeType::Tool
    }
}
