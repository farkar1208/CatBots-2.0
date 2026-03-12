use crate::registry::TagSchemaRegistry;
use crate::storage::TagStorage;
use crate::types::{InstanceId, NodeId, Result, TagId, TagInstance, TagStatus};

/// NodeStorage trait（用于获取节点索引）
#[async_trait::async_trait]
pub trait NodeStorage: Send + Sync {
    async fn get_child_index(&self, parent_id: &NodeId, child_id: &NodeId) -> Result<usize>;
    async fn get_children_count(&self, parent_id: &NodeId) -> Result<usize>;
}

/// 内存实现
pub mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Clone)]
    pub struct MemoryNodeStorage {
        children: Arc<RwLock<HashMap<NodeId, Vec<NodeId>>>>,
    }

    impl MemoryNodeStorage {
        pub fn new() -> Self {
            Self {
                children: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn add_child(&self, parent_id: &NodeId, child_id: &NodeId) {
            let mut children = self.children.write().await;
            children
                .entry(parent_id.clone())
                .or_default()
                .push(child_id.clone());
        }
    }

    #[async_trait::async_trait]
    impl NodeStorage for MemoryNodeStorage {
        async fn get_child_index(&self, parent_id: &NodeId, child_id: &NodeId) -> Result<usize> {
            let children = self.children.read().await;
            let list = children.get(parent_id).cloned().unwrap_or_default();
            list.iter()
                .position(|id| id == child_id)
                .ok_or_else(|| crate::types::TagError::NotFound(format!(
                    "Child {} not found under parent {}",
                    child_id, parent_id
                )))
        }

        async fn get_children_count(&self, parent_id: &NodeId) -> Result<usize> {
            let children = self.children.read().await;
            Ok(children.get(parent_id).map(|v| v.len()).unwrap_or(0))
        }
    }
}

pub struct TagInheritance<S, R, N> {
    storage: S,
    registry: R,
    node_storage: N,
}

impl<S, R, N> TagInheritance<S, R, N>
where
    S: TagStorage,
    R: TagSchemaRegistry,
    N: NodeStorage,
{
    pub fn new(storage: S, registry: R, node_storage: N) -> Self {
        Self {
            storage,
            registry,
            node_storage,
        }
    }

    /// F2: 计算标签继承列表
    pub async fn calculate_inheritance_list(
        &self,
        parent_id: &NodeId,
        child_index: usize,
    ) -> Result<Vec<TagId>> {
        let parent_tags = self
            .storage
            .get_node_tags(parent_id, Some(TagStatus::Active))
            .await?;
        let mut result = Vec::new();

        for tag in parent_tags {
            let rules = self
                .registry
                .get_inheritance_rules(&tag.tag_id, child_index == 0)
                .await?;
            result.extend(rules);
        }

        Ok(result)
    }

    /// F3: 创建继承的标签实例
    pub async fn create_inherited_tags(
        &self,
        parent_id: &NodeId,
        child_id: &NodeId,
        child_index: usize,
    ) -> Result<Vec<TagInstance>> {
        // 1. 计算继承列表
        let tag_ids = self.calculate_inheritance_list(parent_id, child_index).await?;

        // 2. 创建标签实例
        let mut instances = Vec::new();
        for tag_id in tag_ids {
            let instance = self.storage.create_tag_instance(child_id, &tag_id).await?;
            instances.push(instance);
        }

        Ok(instances)
    }

    /// F5: 复制非处理标签到子节点
    pub async fn copy_non_processed_tags(
        &self,
        parent_id: &NodeId,
        child_id: &NodeId,
        exclude_instance_id: &InstanceId,
    ) -> Result<Vec<TagInstance>> {
        let parent_tags = self
            .storage
            .get_node_tags(parent_id, Some(TagStatus::Active))
            .await?;
        let mut instances = Vec::new();

        for tag in parent_tags {
            if tag.instance_id != *exclude_instance_id {
                let instance = self
                    .storage
                    .copy_tag_instance(&tag.instance_id, child_id)
                    .await?;
                instances.push(instance);
            }
        }

        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::memory::MemoryNodeStorage;
    use crate::registry::memory::MemoryTagSchemaRegistry;
    use crate::storage::memory::MemoryTagStorage;
    use crate::types::TagSchema;

    async fn setup() -> (MemoryTagStorage, MemoryTagSchemaRegistry, MemoryNodeStorage) {
        (
            MemoryTagStorage::new(),
            MemoryTagSchemaRegistry::new(),
            MemoryNodeStorage::new(),
        )
    }

    #[tokio::test]
    async fn test_calculate_inheritance_list() {
        let (storage, registry, node_storage) = setup().await;
        let inheritance = TagInheritance::new(storage.clone(), registry.clone(), node_storage);

        // 注册父标签
        registry
            .register(TagSchema {
                tag_id: "parent_tag".to_string(),
                display_name: "Parent".to_string(),
                expire_at: None,
                on_first_child: vec!["inherited_a".to_string()],
                on_branch_child: vec!["inherited_b".to_string()],
                resolver_node_type: None,
                is_blocking: false,
            })
            .await;

        // 创建父标签实例
        storage
            .create_tag_instance(&"node_1".to_string(), &"parent_tag".to_string())
            .await
            .unwrap();

        // 长子继承
        let first_child_tags = inheritance
            .calculate_inheritance_list(&"node_1".to_string(), 0)
            .await
            .unwrap();
        assert_eq!(first_child_tags, vec!["inherited_a".to_string()]);

        // 分支继承
        let branch_tags = inheritance
            .calculate_inheritance_list(&"node_1".to_string(), 1)
            .await
            .unwrap();
        assert_eq!(branch_tags, vec!["inherited_b".to_string()]);
    }

    #[tokio::test]
    async fn test_copy_non_processed_tags() {
        let (storage, registry, node_storage) = setup().await;
        let inheritance = TagInheritance::new(storage.clone(), registry.clone(), node_storage);

        // 创建两个标签实例
        let instance1 = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();
        let instance2 = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_2".to_string())
            .await
            .unwrap();

        // 复制非处理标签（排除instance1）
        let copied = inheritance
            .copy_non_processed_tags(&"node_1".to_string(), &"node_2".to_string(), &instance1.instance_id)
            .await
            .unwrap();

        assert_eq!(copied.len(), 1);
        assert_eq!(copied[0].tag_id, "tag_2".to_string());
    }
}
