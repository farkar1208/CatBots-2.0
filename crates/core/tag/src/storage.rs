use async_trait::async_trait;

use crate::types::{InstanceId, NodeId, Result, TagError, TagId, TagInstance, TagStatus};

/// 标签存储trait
#[async_trait]
pub trait TagStorage: Send + Sync {
    /// S1: 获取节点所有标签实例（可筛选状态）
    async fn get_node_tags(
        &self,
        node_id: &NodeId,
        status_filter: Option<TagStatus>,
    ) -> Result<Vec<TagInstance>>;

    /// S2: 获取指定标签实例
    async fn get_tag_instance(&self, instance_id: &InstanceId) -> Result<TagInstance>;

    /// S3: 创建标签实例
    async fn create_tag_instance(
        &self,
        node_id: &NodeId,
        tag_id: &TagId,
    ) -> Result<TagInstance>;

    /// S4: 复制标签实例到新节点
    async fn copy_tag_instance(
        &self,
        source_instance_id: &InstanceId,
        target_node_id: &NodeId,
    ) -> Result<TagInstance>;

    /// S5: 更新标签状态
    async fn update_tag_status(
        &self,
        instance_id: &InstanceId,
        new_status: TagStatus,
    ) -> Result<()>;
}

/// 内存实现（用于测试）
pub mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Clone)]
    pub struct MemoryTagStorage {
        instances: Arc<RwLock<HashMap<InstanceId, TagInstance>>>,
        node_tags: Arc<RwLock<HashMap<NodeId, Vec<InstanceId>>>>,
        counter: Arc<RwLock<u64>>,
    }

    impl MemoryTagStorage {
        pub fn new() -> Self {
            Self {
                instances: Arc::new(RwLock::new(HashMap::new())),
                node_tags: Arc::new(RwLock::new(HashMap::new())),
                counter: Arc::new(RwLock::new(0)),
            }
        }

        async fn next_id(&self) -> InstanceId {
            let mut counter = self.counter.write().await;
            *counter += 1;
            format!("inst_{}", counter)
        }
    }

    #[async_trait]
    impl TagStorage for MemoryTagStorage {
        async fn get_node_tags(
            &self,
            node_id: &NodeId,
            status_filter: Option<TagStatus>,
        ) -> Result<Vec<TagInstance>> {
            let node_tags = self.node_tags.read().await;
            let instances = self.instances.read().await;

            let instance_ids = node_tags.get(node_id).cloned().unwrap_or_default();

            let mut result = Vec::new();
            for id in instance_ids {
                if let Some(instance) = instances.get(&id) {
                    if let Some(filter) = status_filter {
                        if instance.status == filter {
                            result.push(instance.clone());
                        }
                    } else {
                        result.push(instance.clone());
                    }
                }
            }

            Ok(result)
        }

        async fn get_tag_instance(&self, instance_id: &InstanceId) -> Result<TagInstance> {
            let instances = self.instances.read().await;
            instances
                .get(instance_id)
                .cloned()
                .ok_or_else(|| TagError::NotFound(format!("Instance {}", instance_id)))
        }

        async fn create_tag_instance(
            &self,
            node_id: &NodeId,
            tag_id: &TagId,
        ) -> Result<TagInstance> {
            let instance_id = self.next_id().await;
            let instance = TagInstance {
                instance_id: instance_id.clone(),
                node_id: node_id.clone(),
                tag_id: tag_id.clone(),
                created_time: chrono::Utc::now().timestamp(),
                status: TagStatus::Active,
            };

            let mut instances = self.instances.write().await;
            instances.insert(instance_id.clone(), instance.clone());

            let mut node_tags = self.node_tags.write().await;
            node_tags
                .entry(node_id.clone())
                .or_default()
                .push(instance_id);

            Ok(instance)
        }

        async fn copy_tag_instance(
            &self,
            source_instance_id: &InstanceId,
            target_node_id: &NodeId,
        ) -> Result<TagInstance> {
            let source = self.get_tag_instance(source_instance_id).await?;
            self.create_tag_instance(target_node_id, &source.tag_id)
                .await
        }

        async fn update_tag_status(
            &self,
            instance_id: &InstanceId,
            new_status: TagStatus,
        ) -> Result<()> {
            let mut instances = self.instances.write().await;
            let instance = instances
                .get_mut(instance_id)
                .ok_or_else(|| TagError::NotFound(format!("Instance {}", instance_id)))?;
            instance.status = new_status;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::memory::MemoryTagStorage;

    #[tokio::test]
    async fn test_create_and_get_tag_instance() {
        let storage = MemoryTagStorage::new();
        let node_id = "node_1".to_string();
        let tag_id = "tag_1".to_string();

        let instance = storage
            .create_tag_instance(&node_id, &tag_id)
            .await
            .unwrap();

        assert_eq!(instance.node_id, node_id);
        assert_eq!(instance.tag_id, tag_id);
        assert_eq!(instance.status, TagStatus::Active);

        let fetched = storage
            .get_tag_instance(&instance.instance_id)
            .await
            .unwrap();
        assert_eq!(fetched.instance_id, instance.instance_id);
    }

    #[tokio::test]
    async fn test_get_node_tags() {
        let storage = MemoryTagStorage::new();
        let node_id = "node_1".to_string();

        storage
            .create_tag_instance(&node_id, &"tag_1".to_string())
            .await
            .unwrap();
        storage
            .create_tag_instance(&node_id, &"tag_2".to_string())
            .await
            .unwrap();

        let tags = storage.get_node_tags(&node_id, None).await.unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn test_update_tag_status() {
        let storage = MemoryTagStorage::new();
        let node_id = "node_1".to_string();

        let instance = storage
            .create_tag_instance(&node_id, &"tag_1".to_string())
            .await
            .unwrap();

        storage
            .update_tag_status(&instance.instance_id, TagStatus::Revoked)
            .await
            .unwrap();

        let tags = storage
            .get_node_tags(&node_id, Some(TagStatus::Active))
            .await
            .unwrap();
        assert_eq!(tags.len(), 0);

        let tags = storage
            .get_node_tags(&node_id, Some(TagStatus::Revoked))
            .await
            .unwrap();
        assert_eq!(tags.len(), 1);
    }
}
