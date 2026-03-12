//! Tag 持久化存储实现
//!
//! 使用 catbots-persistence crate 提供的 PersistenceService 进行数据持久化

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::TagStorage;
use crate::types::{InstanceId, NodeId, Result, TagError, TagId, TagInstance, TagStatus};

/// Tag 存储数据（用于序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagStorageData {
    /// 所有标签实例
    instances: HashMap<InstanceId, TagInstance>,
    /// 节点到标签实例的映射
    node_tags: HashMap<NodeId, Vec<InstanceId>>,
}

impl Default for TagStorageData {
    fn default() -> Self {
        Self {
            instances: HashMap::new(),
            node_tags: HashMap::new(),
        }
    }
}

/// 持久化 TagStorage 实现
pub struct PersistentTagStorage<P> {
    persistence: P,
    storage_key: String,
}

impl<P: catbots_persistence::PersistenceService> PersistentTagStorage<P> {
    pub fn new(persistence: P, storage_key: impl Into<String>) -> Self {
        Self {
            persistence,
            storage_key: storage_key.into(),
        }
    }

    /// 加载存储数据
    async fn load_data(&self) -> Result<TagStorageData> {
        let data: Option<TagStorageData> = self
            .persistence
            .load(&self.storage_key)
            .await
            .map_err(|e| TagError::StorageError(e.to_string()))?;
        Ok(data.unwrap_or_default())
    }

    /// 保存存储数据
    async fn save_data(&self, data: &TagStorageData) -> Result<()> {
        self.persistence
            .save(&self.storage_key, data)
            .await
            .map_err(|e| TagError::StorageError(e.to_string()))
    }

    /// 生成唯一实例ID
    fn generate_instance_id(&self) -> InstanceId {
        format!("inst_{}", uuid::Uuid::new_v4())
    }
}

#[async_trait]
impl<P: catbots_persistence::PersistenceService + Send + Sync> TagStorage for PersistentTagStorage<P> {
    async fn get_node_tags(
        &self,
        node_id: &NodeId,
        status_filter: Option<TagStatus>,
    ) -> Result<Vec<TagInstance>> {
        let data = self.load_data().await?;

        let instance_ids = data.node_tags.get(node_id).cloned().unwrap_or_default();

        let mut result = Vec::new();
        for id in instance_ids {
            if let Some(instance) = data.instances.get(&id) {
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
        let data = self.load_data().await?;
        data.instances
            .get(instance_id)
            .cloned()
            .ok_or_else(|| TagError::NotFound(format!("Instance {}", instance_id)))
    }

    async fn create_tag_instance(
        &self,
        node_id: &NodeId,
        tag_id: &TagId,
    ) -> Result<TagInstance> {
        let mut data = self.load_data().await?;

        let instance_id = self.generate_instance_id();
        let instance = TagInstance {
            instance_id: instance_id.clone(),
            node_id: node_id.clone(),
            tag_id: tag_id.clone(),
            created_time: chrono::Utc::now().timestamp(),
            status: TagStatus::Active,
        };

        // 添加到实例表
        data.instances.insert(instance_id.clone(), instance.clone());

        // 添加到节点映射
        data.node_tags
            .entry(node_id.clone())
            .or_default()
            .push(instance_id);

        // 保存
        self.save_data(&data).await?;

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
        let mut data = self.load_data().await?;

        let instance = data
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| TagError::NotFound(format!("Instance {}", instance_id)))?;

        instance.status = new_status;

        self.save_data(&data).await?;
        Ok(())
    }
}

/// TagSchema 存储数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SchemaStorageData {
    schemas: HashMap<TagId, crate::types::TagSchema>,
}

/// 持久化 TagSchemaRegistry 实现
pub struct PersistentTagSchemaRegistry<P> {
    persistence: P,
    storage_key: String,
}

impl<P: catbots_persistence::PersistenceService> PersistentTagSchemaRegistry<P> {
    pub fn new(persistence: P, storage_key: impl Into<String>) -> Self {
        Self {
            persistence,
            storage_key: storage_key.into(),
        }
    }

    /// 加载存储数据
    async fn load_data(&self) -> Result<SchemaStorageData> {
        let data: Option<SchemaStorageData> = self
            .persistence
            .load(&self.storage_key)
            .await
            .map_err(|e| TagError::StorageError(e.to_string()))?;
        Ok(data.unwrap_or_default())
    }

    /// 保存存储数据
    async fn save_data(&self, data: &SchemaStorageData) -> Result<()> {
        self.persistence
            .save(&self.storage_key, data)
            .await
            .map_err(|e| TagError::StorageError(e.to_string()))
    }

    /// 注册标签定义（用于初始化）
    pub async fn register(&self, schema: crate::types::TagSchema) -> Result<()> {
        let mut data = self.load_data().await?;
        data.schemas.insert(schema.tag_id.clone(), schema);
        self.save_data(&data).await
    }
}

#[async_trait]
impl<P: catbots_persistence::PersistenceService + Send + Sync> crate::registry::TagSchemaRegistry
    for PersistentTagSchemaRegistry<P>
{
    async fn get_tag_schema(&self, tag_id: &TagId) -> Result<crate::types::TagSchema> {
        let data = self.load_data().await?;
        data.schemas
            .get(tag_id)
            .cloned()
            .ok_or_else(|| TagError::RegistryError(format!("Schema not found: {}", tag_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catbots_persistence::MemoryPersistence;
    use crate::registry::TagSchemaRegistry;
    use crate::types::TagSchema;

    #[tokio::test]
    async fn test_persistent_storage_create_and_get() {
        let persistence = MemoryPersistence::new();
        let storage = PersistentTagStorage::new(persistence, "test_tags");

        let node_id = "node_1".to_string();
        let tag_id = "tag_1".to_string();

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&node_id, &tag_id)
            .await
            .unwrap();

        assert_eq!(instance.node_id, node_id);
        assert_eq!(instance.tag_id, tag_id);

        // 获取标签实例
        let fetched = storage.get_tag_instance(&instance.instance_id).await.unwrap();
        assert_eq!(fetched.instance_id, instance.instance_id);

        // 获取节点标签
        let tags = storage.get_node_tags(&node_id, None).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag_id, tag_id);
    }

    #[tokio::test]
    async fn test_persistent_storage_update_status() {
        let persistence = MemoryPersistence::new();
        let storage = PersistentTagStorage::new(persistence, "test_tags");

        let node_id = "node_1".to_string();
        let instance = storage
            .create_tag_instance(&node_id, &"tag_1".to_string())
            .await
            .unwrap();

        // 更新状态
        storage
            .update_tag_status(&instance.instance_id, TagStatus::Revoked)
            .await
            .unwrap();

        // 验证
        let active_tags = storage
            .get_node_tags(&node_id, Some(TagStatus::Active))
            .await
            .unwrap();
        assert_eq!(active_tags.len(), 0);

        let revoked_tags = storage
            .get_node_tags(&node_id, Some(TagStatus::Revoked))
            .await
            .unwrap();
        assert_eq!(revoked_tags.len(), 1);
    }

    #[tokio::test]
    async fn test_persistent_registry() {
        let persistence = MemoryPersistence::new();
        let registry = PersistentTagSchemaRegistry::new(persistence, "test_schemas");

        let schema = TagSchema {
            tag_id: "test_tag".to_string(),
            display_name: "Test Tag".to_string(),
            expire_at: None,
            on_first_child: vec![],
            on_branch_child: vec![],
            resolver_node_type: None,
            is_blocking: false,
        };

        // 注册
        registry.register(schema.clone()).await.unwrap();

        // 获取
        let fetched = registry.get_tag_schema(&"test_tag".to_string()).await.unwrap();
        assert_eq!(fetched.tag_id, schema.tag_id);
        assert_eq!(fetched.display_name, schema.display_name);
    }

    #[tokio::test]
    async fn test_persistent_storage_persistence() {
        // 使用同一个 persistence 实例，验证数据是否真正持久化
        let persistence = MemoryPersistence::new();

        // 第一个 storage 实例创建数据
        {
            let storage = PersistentTagStorage::new(persistence.clone(), "shared_tags");
            storage
                .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
                .await
                .unwrap();
        }

        // 第二个 storage 实例读取数据
        {
            let storage = PersistentTagStorage::new(persistence.clone(), "shared_tags");
            let tags = storage.get_node_tags(&"node_1".to_string(), None).await.unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].tag_id, "tag_1".to_string());
        }
    }
}
