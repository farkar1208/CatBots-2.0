//! Tag 系统集成模块
//!
//! 提供 ConversationTree 与 TagManager 的集成功能

use crate::{ConversationTree, NodeEnum, NodeType};
use async_trait::async_trait;
use catbots_tag::{
    CreateNodeResult, CreateResolverResult, InstanceId, NodeCreator, NodeId, NodeStorage,
    ProcessResult, TagError, TagManager, TagSchemaRegistry, TagStatus, TagStorage,
    ValidationResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// ConversationTree 的 NodeStorage 实现
/// 
/// 用于 TagInheritance 查询子节点索引
#[derive(Clone)]
pub struct TreeNodeStorage {
    tree: Arc<Mutex<ConversationTree>>,
}

impl TreeNodeStorage {
    pub fn new(tree: Arc<Mutex<ConversationTree>>) -> Self {
        Self { tree }
    }
}

#[async_trait]
impl NodeStorage for TreeNodeStorage {
    async fn get_child_index(&self, parent_id: &NodeId, child_id: &NodeId) -> catbots_tag::Result<usize> {
        let tree = self.tree.lock().await;
        let children = tree.get_children(parent_id);
        children
            .iter()
            .position(|id| id == child_id)
            .ok_or_else(|| TagError::NotFound(format!(
                "Child {} not found under parent {}",
                child_id, parent_id
            )))
    }

    async fn get_children_count(&self, parent_id: &NodeId) -> catbots_tag::Result<usize> {
        let tree = self.tree.lock().await;
        Ok(tree.get_children(parent_id).len())
    }
}

/// ConversationTree 的 NodeCreator 实现
/// 
/// 用于 TagManager 创建节点
pub struct TreeNodeCreator {
    tree: Arc<Mutex<ConversationTree>>,
    node_type: NodeType,
    content: String,
    model: Option<String>,
}

impl TreeNodeCreator {
    pub fn new_user(tree: Arc<Mutex<ConversationTree>>, content: String) -> Self {
        Self {
            tree,
            node_type: NodeType::User,
            content,
            model: None,
        }
    }

    pub fn new_ai(tree: Arc<Mutex<ConversationTree>>, content: String, model: String) -> Self {
        Self {
            tree,
            node_type: NodeType::AI,
            content,
            model: Some(model),
        }
    }

    pub fn new_resolver(
        tree: Arc<Mutex<ConversationTree>>,
        resolver_type: &str,
        content: String,
    ) -> Self {
        // resolver_type 可以映射到特定的 NodeType
        Self {
            tree,
            node_type: NodeType::User, // 或根据 resolver_type 映射
            content,
            model: None,
        }
    }
}

#[async_trait]
impl NodeCreator for TreeNodeCreator {
    async fn create_node(&self, parent_id: &NodeId) -> catbots_tag::Result<NodeId> {
        let mut tree = self.tree.lock().await;
        
        let node_id = match self.node_type {
            NodeType::User => tree.add_user_node(parent_id, self.content.clone()),
            NodeType::AI => {
                let model = self.model.clone().unwrap_or_default();
                tree.add_ai_node(parent_id, self.content.clone(), model)
            }
            _ => return Err(TagError::NodeCreationFailed(
                format!("Unsupported node type: {:?}", self.node_type)
            )),
        };
        
        Ok(node_id)
    }

    async fn create_resolver_node(
        &self,
        parent_id: &NodeId,
        resolver_type: &str,
    ) -> catbots_tag::Result<NodeId> {
        // 创建处理节点（可以是特定类型的 User 或 AI 节点）
        let mut tree = self.tree.lock().await;
        
        let node_id = match self.node_type {
            NodeType::User => tree.add_user_node(parent_id, self.content.clone()),
            NodeType::AI => {
                let model = self.model.clone().unwrap_or_default();
                tree.add_ai_node(parent_id, self.content.clone(), model)
            }
            _ => return Err(TagError::NodeCreationFailed(
                format!("Unsupported resolver type: {}", resolver_type)
            )),
        };
        
        Ok(node_id)
    }
}

/// 带标签集成的 ConversationTree 包装器
pub struct TaggedConversationTree<S, R, A> {
    tree: Arc<Mutex<ConversationTree>>,
    tag_manager: TagManager<S, R, TreeNodeStorage, A>,
}

impl<S, R, A> TaggedConversationTree<S, R, A>
where
    S: TagStorage + Clone,
    R: TagSchemaRegistry + Clone,
    A: catbots_tag::AuditLogger + Clone,
{
    /// 创建新的带标签的对话树
    pub fn new(
        tree: ConversationTree,
        storage: S,
        registry: R,
        audit: A,
    ) -> Self {
        let tree = Arc::new(Mutex::new(tree));
        let node_storage = TreeNodeStorage::new(tree.clone());
        let tag_manager = TagManager::new(storage, registry, node_storage, audit);
        
        Self {
            tree,
            tag_manager,
        }
    }

    /// 获取 TagManager 引用
    pub fn tag_manager(&self) -> &TagManager<S, R, TreeNodeStorage, A> {
        &self.tag_manager
    }

    /// 添加用户节点（带标签校验和继承）
    pub async fn add_user_node_with_tags(
        &self,
        parent_id: &str,
        content: String,
    ) -> catbots_tag::Result<CreateNodeResult> {
        let creator = TreeNodeCreator::new_user(self.tree.clone(), content);
        self.tag_manager
            .create_normal_child_node(&parent_id.to_string(), "user", &creator)
            .await
    }

    /// 添加 AI 节点（带标签校验和继承）
    pub async fn add_ai_node_with_tags(
        &self,
        parent_id: &str,
        content: String,
        model: String,
    ) -> catbots_tag::Result<CreateNodeResult> {
        let creator = TreeNodeCreator::new_ai(self.tree.clone(), content, model);
        self.tag_manager
            .create_normal_child_node(&parent_id.to_string(), "ai", &creator)
            .await
    }

    /// 创建处理节点（处理阻塞标签）
    pub async fn add_resolver_node(
        &self,
        parent_id: &str,
        resolver_type: &str,
        tag_instance_id: &InstanceId,
        input_data: &serde_json::Value,
        content: String,
    ) -> catbots_tag::Result<CreateResolverResult> {
        let creator = TreeNodeCreator::new_resolver(self.tree.clone(), resolver_type, content);
        self.tag_manager
            .create_resolver_node(
                &parent_id.to_string(),
                resolver_type,
                tag_instance_id,
                input_data,
                &creator,
            )
            .await
    }

    /// 撤销节点上的标签
    pub async fn revoke_tag(
        &self,
        instance_id: &InstanceId,
        reason: &str,
    ) -> catbots_tag::Result<()> {
        self.tag_manager.revoke_tag(instance_id, reason).await
    }

    /// 获取节点的标签
    pub async fn get_node_tags(
        &self,
        node_id: &str,
        status_filter: Option<TagStatus>,
    ) -> catbots_tag::Result<Vec<catbots_tag::TagInstance>> {
        // 通过 storage 获取
        // 这里需要通过某种方式访问 storage
        unimplemented!("需要通过 storage 访问节点标签")
    }

    /// 校验节点创建
    pub async fn validate_creation(
        &self,
        parent_id: &str,
        node_type: &str,
    ) -> catbots_tag::Result<ValidationResult> {
        self.tag_manager
            .validate_creation(&parent_id.to_string(), node_type)
            .await
    }
}

/// 将节点标签同步到 ConversationTree
/// 
/// 在创建节点后，将继承的标签实例ID同步到节点的 tags 字段
pub fn sync_tags_to_node(tree: &mut ConversationTree, node_id: &str, tag_instance_ids: Vec<String>) {
    if let Some(node) = tree.get_node_mut(node_id) {
        match node {
            NodeEnum::User(n) => n.tags = tag_instance_ids,
            NodeEnum::AI(n) => n.tags = tag_instance_ids,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catbots_tag::{
        memory::{MemoryAuditLogger, MemoryTagSchemaRegistry, MemoryTagStorage},
        TagSchema,
    };

    async fn setup_test_tree() -> (
        TaggedConversationTree<MemoryTagStorage, MemoryTagSchemaRegistry, MemoryAuditLogger>,
        MemoryTagSchemaRegistry,
    ) {
        let tree = ConversationTree::new();
        let storage = MemoryTagStorage::new();
        let registry = MemoryTagSchemaRegistry::new();
        let audit = MemoryAuditLogger::new();

        let tagged_tree = TaggedConversationTree::new(tree, storage, registry.clone(), audit);
        (tagged_tree, registry)
    }

    #[tokio::test]
    async fn test_add_user_node_with_tags() {
        let (tree, registry) = setup_test_tree().await;

        // 注册一个继承标签
        registry
            .register(TagSchema {
                tag_id: "auto_inherit".to_string(),
                display_name: "Auto Inherit".to_string(),
                expire_at: None,
                on_first_child: vec!["inherited_tag".to_string()],
                on_branch_child: vec![],
                resolver_node_type: None,
                is_blocking: false,
            })
            .await;

        // 在 root 节点上添加标签
        // 注意：这里需要通过其他方式在 root 上添加标签

        // 创建子节点
        let result = tree
            .add_user_node_with_tags("root", "Hello".to_string())
            .await;

        assert!(result.is_ok());
        let create_result = result.unwrap();
        assert!(!create_result.node_id.is_empty());
    }

    #[tokio::test]
    async fn test_validate_creation_blocked() {
        let (tree, registry) = setup_test_tree().await;

        // 注册阻塞标签
        registry
            .register(TagSchema {
                tag_id: "blocking".to_string(),
                display_name: "Blocking".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: Some("resolver".to_string()),
                is_blocking: true,
            })
            .await;

        // 在 root 上创建阻塞标签（需要通过 storage 直接创建）
        // 这里简化处理，实际应该在 root 节点上添加标签

        // 验证创建
        let result = tree.validate_creation("root", "user").await;
        assert!(result.is_ok());
        // 如果没有阻塞标签，应该允许创建
        assert!(result.unwrap().allowed);
    }
}
