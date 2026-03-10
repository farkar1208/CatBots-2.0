//! History 模块 - 对话树存储 + 处理协调
//!
//! 核心职责：
//! - `ConversationTree`: 数据存储中心 - 节点注册、获取、上下文构建
//! - `NodeProcessor`: 处理协调中心 - 构建 Task、分发 Handler、写回结果
//! - `Handler`: 处理器 trait - 纯粹的处理逻辑，不依赖 Tree
//! - `Node` 系列: 节点数据结构
//! - `Message`: 消息结构

mod conversation_tree;
mod handler;
mod message;
mod node;
mod node_processor;

pub use conversation_tree::{ConversationTree, NodeEnum};
pub use handler::{
    AIResult, AITask, Handler, ModelPreferences, ResultBase, ResultData, SamplingResult,
    SamplingTask, Task, TaskBase, TokenUsage,
};
pub use message::{Message, MessageRole};
pub use node::{AINode, Node, NodeType, RootNode, TokenUsage as NodeTokenUsage, ToolNode, UserNode};
pub use node_processor::NodeProcessor;
