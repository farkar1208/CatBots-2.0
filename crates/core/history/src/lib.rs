//! History 模块 - 对话树存储 + 上下文构建
//!
//! 核心职责：
//! - `ConversationTree`: 对话树管理 + 上下文构建
//! - `Node` 系列: 节点数据结构
//! - `Message`: 消息结构

mod conversation_tree;
mod message;
mod node;

pub use conversation_tree::{ConversationTree, NodeEnum};
pub use message::{Message, MessageRole};
pub use node::{AINode, Node, NodeType, RootNode, ToolNode, UserNode};
