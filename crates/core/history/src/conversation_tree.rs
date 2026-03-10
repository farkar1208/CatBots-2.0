//! 对话树 - 核心数据结构

use crate::{AINode, Message, Node, NodeType, RootNode, UserNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 对话树
/// 
/// 核心职责：
/// - 管理对话节点
/// - 构建上下文
/// - 支持分支对话
/// - 支持持久化存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTree {
    /// 根节点
    root: RootNode,
    /// 节点索引 (node_id -> Node)
    node_index: HashMap<String, NodeEnum>,
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    max_tokens: usize,
}

fn default_max_tokens() -> usize { 4096 }

/// 节点枚举 - 用于统一存储不同类型的节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeEnum {
    Root(RootNode),
    User(UserNode),
    AI(AINode),
    // Tool(ToolNode), // TODO: 添加工具节点支持
}

impl NodeEnum {
    /// 获取节点ID
    pub fn id(&self) -> &str {
        match self {
            NodeEnum::Root(n) => n.id(),
            NodeEnum::User(n) => n.id(),
            NodeEnum::AI(n) => n.id(),
        }
    }

    /// 获取父节点ID
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            NodeEnum::Root(_) => None,
            NodeEnum::User(n) => Some(&n.parent_id),
            NodeEnum::AI(n) => Some(&n.parent_id),
        }
    }

    /// 获取子节点ID列表
    pub fn children(&self) -> &[String] {
        match self {
            NodeEnum::Root(n) => n.children(),
            NodeEnum::User(n) => n.children(),
            NodeEnum::AI(n) => n.children(),
        }
    }

    /// 添加子节点
    pub fn add_child(&mut self, child_id: String) {
        match self {
            NodeEnum::Root(n) => n.add_child(child_id),
            NodeEnum::User(n) => n.add_child(child_id),
            NodeEnum::AI(n) => n.add_child(child_id),
        }
    }

    /// 转换为消息（仅 User 和 AI 节点）
    pub fn to_message(&self) -> Option<Message> {
        match self {
            NodeEnum::Root(_) => None,
            NodeEnum::User(n) => Some(Message::user(&n.content)),
            NodeEnum::AI(n) => Some(Message::assistant(&n.content)),
        }
    }

    /// 获取节点类型
    pub fn node_type(&self) -> NodeType {
        match self {
            NodeEnum::Root(_) => NodeType::Root,
            NodeEnum::User(_) => NodeType::User,
            NodeEnum::AI(_) => NodeType::AI,
        }
    }
}

impl ConversationTree {
    /// 创建新的对话树
    pub fn new() -> Self {
        Self {
            root: RootNode::new(),
            node_index: HashMap::new(),
            max_tokens: 4096,
        }
    }

    /// 设置最大 token 数
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// 获取节点（返回克隆）
    pub fn get_node(&self, id: &str) -> Option<NodeEnum> {
        if id == "root" {
            Some(NodeEnum::Root(self.root.clone()))
        } else {
            self.node_index.get(id).cloned()
        }
    }

    /// 添加用户节点
    /// 
    /// # 参数
    /// - `parent_id`: 父节点ID
    /// - `content`: 用户消息内容
    /// 
    /// # 返回
    /// 新创建的节点ID
    pub fn add_user_node(&mut self, parent_id: &str, content: String) -> String {
        let node = UserNode::new(parent_id.to_string(), content);
        let node_id = node.id.clone();
        
        // 更新父节点的 children
        if parent_id == "root" {
            self.root.add_child(node_id.clone());
        } else if let Some(parent) = self.node_index.get_mut(parent_id) {
            parent.add_child(node_id.clone());
        }
        
        self.node_index.insert(node_id.clone(), NodeEnum::User(node));
        node_id
    }

    /// 添加 AI 节点
    /// 
    /// # 参数
    /// - `parent_id`: 父节点ID
    /// - `content`: AI 响应内容
    /// - `model`: 使用的模型名称
    /// 
    /// # 返回
    /// 新创建的节点ID
    pub fn add_ai_node(&mut self, parent_id: &str, content: String, model: String) -> String {
        let node = AINode::new(parent_id.to_string(), content, model);
        let node_id = node.id.clone();
        
        // 更新父节点的 children
        if parent_id == "root" {
            self.root.add_child(node_id.clone());
        } else if let Some(parent) = self.node_index.get_mut(parent_id) {
            parent.add_child(node_id.clone());
        }
        
        self.node_index.insert(node_id.clone(), NodeEnum::AI(node));
        node_id
    }

    /// 获取从根节点到指定节点的路径
    /// 
    /// 返回从 root 到目标节点的节点ID列表（包含两端）
    /// 路径顺序：[root, ..., target]
    pub fn get_path(&self, node_id: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut current_id = node_id.to_string();
        
        // 从目标节点向上回溯到 root
        let mut visited = std::collections::HashSet::new();
        
        loop {
            if visited.contains(&current_id) {
                // 防止循环（不应该发生，但安全起见）
                break;
            }
            visited.insert(current_id.clone());
            
            path.push(current_id.clone());
            
            if current_id == "root" {
                break;
            }
            
            // 获取父节点ID
            let parent = if current_id == node_id {
                self.get_node(&current_id)
            } else {
                self.node_index.get(&current_id).cloned()
            };
            
            match parent {
                Some(node) => {
                    match node.parent_id() {
                        Some(pid) => current_id = pid.to_string(),
                        None => break, // root 节点
                    }
                }
                None => break, // 节点不存在
            }
        }
        
        // 反转路径，使其从 root 开始
        path.reverse();
        path
    }

    /// 获取上下文消息列表
    /// 
    /// # 参数
    /// - `node_id`: 目标节点ID
    /// 
    /// # 返回
    /// 从 root 到目标节点的消息列表（不包含 root）
    pub fn get_context(&self, node_id: &str) -> Vec<Message> {
        let path = self.get_path(node_id);
        let messages = self.build_context(&path);
        self.truncate(messages)
    }

    /// 获取子节点列表
    pub fn get_children(&self, node_id: &str) -> Vec<&str> {
        if node_id == "root" {
            self.root.children.iter().map(|s| s.as_str()).collect()
        } else {
            self.node_index
                .get(node_id)
                .map(|n| n.children().iter().map(|s| s.as_str()).collect())
                .unwrap_or_default()
        }
    }

    /// 构建上下文（内部方法）
    /// 
    /// 将节点路径转换为消息列表
    fn build_context(&self, path: &[String]) -> Vec<Message> {
        path.iter()
            .filter_map(|id| {
                if id == "root" {
                    return None;
                }
                self.node_index.get(id).and_then(|n| n.to_message())
            })
            .collect()
    }

    /// 截断消息列表以适应 token 限制（内部方法）
    /// 
    /// 使用简单的字符估算：4 字符 ≈ 1 token
    /// 从消息列表开头开始截断，保留最近的对话
    fn truncate(&self, messages: Vec<Message>) -> Vec<Message> {
        if messages.is_empty() {
            return messages;
        }

        // 计算每条消息的 token 估算值
        let msg_tokens: Vec<(usize, &Message)> = messages
            .iter()
            .map(|m| {
                // 简单估算：4 字符 ≈ 1 token
                let tokens = m.content.len() / 4 + 1;
                (tokens, m)
            })
            .collect();

        // 计算总 token 数
        let total_tokens: usize = msg_tokens.iter().map(|(t, _)| *t).sum();

        if total_tokens <= self.max_tokens {
            return messages;
        }

        // 从开头截断，保留最近的对话
        let mut result = Vec::new();
        let mut current_tokens = 0;

        // 从后向前遍历，收集消息直到达到限制
        for (tokens, msg) in msg_tokens.iter().rev() {
            if current_tokens + tokens > self.max_tokens {
                break;
            }
            result.push((*msg).clone());
            current_tokens += tokens;
        }

        // 反转回正确顺序
        result.reverse();
        result
    }

    /// 获取当前节点ID（用于恢复会话）
    pub fn current_node_id(&self) -> &str {
        // 返回最后一个叶子节点，如果没有则返回 root
        // 这是一个简化的实现
        "root"
    }

    /// 获取节点总数
    pub fn node_count(&self) -> usize {
        self.node_index.len() + 1 // +1 for root
    }

    // ============================================================
    // 持久化方法
    // ============================================================

    /// 保存到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        
        tracing::info!(
            path = %path.display(),
            node_count = self.node_count(),
            "已保存对话历史"
        );
        
        Ok(())
    }

    /// 从文件加载
    pub fn load_from_file(path: &PathBuf) -> Result<Self, anyhow::Error> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)?;
        
        if content.trim().is_empty() {
            return Ok(Self::new());
        }

        let tree: ConversationTree = serde_json::from_str(&content)?;
        
        tracing::info!(
            path = %path.display(),
            node_count = tree.node_count(),
            "已加载对话历史"
        );
        
        Ok(tree)
    }

    /// 清空对话历史
    pub fn clear(&mut self) {
        self.root = RootNode::new();
        self.node_index.clear();
    }
}

impl Default for ConversationTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageRole;

    #[test]
    fn test_add_nodes() {
        let mut tree = ConversationTree::new();
        
        let user1 = tree.add_user_node("root", "Hello".to_string());
        let _ai1 = tree.add_ai_node(&user1, "Hi there!".to_string(), "gpt-4o".to_string());
        let user2 = tree.add_user_node(&_ai1, "How are you?".to_string());
        
        // 检查路径
        let path = tree.get_path(&user2);
        assert_eq!(path.len(), 4); // root -> user1 -> ai1 -> user2
        assert_eq!(path[0], "root");
        assert_eq!(path[1], user1);
        assert_eq!(path[2], _ai1);
        assert_eq!(path[3], user2);
    }

    #[test]
    fn test_get_context() {
        let mut tree = ConversationTree::new();
        
        let user1 = tree.add_user_node("root", "Hello".to_string());
        let ai1 = tree.add_ai_node(&user1, "Hi there!".to_string(), "gpt-4o".to_string());
        
        let context = tree.get_context(&ai1);
        assert_eq!(context.len(), 2);
        assert_eq!(context[0].role, MessageRole::User);
        assert_eq!(context[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_branch() {
        let mut tree = ConversationTree::new();
        
        let user1 = tree.add_user_node("root", "Hello".to_string());
        let _ai1 = tree.add_ai_node(&user1, "Hi!".to_string(), "gpt-4o".to_string());
        
        // 从 user1 分支
        let _user2_branch = tree.add_user_node(&user1, "Different topic".to_string());
        
        // 检查 user1 有两个子节点
        let children = tree.get_children(&user1);
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut tree = ConversationTree::new();
        let user1 = tree.add_user_node("root", "Hello".to_string());
        let _ai1 = tree.add_ai_node(&user1, "Hi there!".to_string(), "gpt-4o".to_string());
        
        // 序列化
        let json = serde_json::to_string(&tree).unwrap();
        
        // 反序列化
        let tree2: ConversationTree = serde_json::from_str(&json).unwrap();
        
        // 验证
        assert_eq!(tree2.node_count(), tree.node_count());
        let path = tree2.get_path(&_ai1);
        assert_eq!(path.len(), 3);
    }
}
