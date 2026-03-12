use async_trait::async_trait;
use std::collections::HashMap;

use crate::types::{Result, TagError, TagId, TagSchema};

/// 标签Schema注册表trait
#[async_trait]
pub trait TagSchemaRegistry: Send + Sync {
    /// S6: 获取标签定义
    async fn get_tag_schema(&self, tag_id: &TagId) -> Result<TagSchema>;

    /// S7: 获取继承规则（组合方法）
    async fn get_inheritance_rules(
        &self,
        tag_id: &TagId,
        is_first_child: bool,
    ) -> Result<Vec<TagId>> {
        let schema = self.get_tag_schema(tag_id).await?;
        Ok(if is_first_child {
            schema.on_first_child
        } else {
            schema.on_branch_child
        })
    }

    /// S8: 获取标签处理者类型
    async fn get_resolver_type(&self, tag_id: &TagId) -> Result<Option<String>> {
        let schema = self.get_tag_schema(tag_id).await?;
        Ok(schema.resolver_node_type)
    }
}

/// 内存实现（用于测试）
pub mod memory {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Clone)]
    pub struct MemoryTagSchemaRegistry {
        schemas: Arc<RwLock<HashMap<TagId, TagSchema>>>,
    }

    impl MemoryTagSchemaRegistry {
        pub fn new() -> Self {
            Self {
                schemas: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub async fn register(&self, schema: TagSchema) {
            let mut schemas = self.schemas.write().await;
            schemas.insert(schema.tag_id.clone(), schema);
        }
    }

    #[async_trait]
    impl TagSchemaRegistry for MemoryTagSchemaRegistry {
        async fn get_tag_schema(&self, tag_id: &TagId) -> Result<TagSchema> {
            let schemas = self.schemas.read().await;
            schemas
                .get(tag_id)
                .cloned()
                .ok_or_else(|| TagError::RegistryError(format!("Schema not found: {}", tag_id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::memory::MemoryTagSchemaRegistry;

    #[tokio::test]
    async fn test_get_tag_schema() {
        let registry = MemoryTagSchemaRegistry::new();
        let schema = TagSchema {
            tag_id: "tag_1".to_string(),
            display_name: "Test Tag".to_string(),
            expire_at: None,
            on_first_child: vec!["tag_2".to_string()],
            on_branch_child: vec![],
            resolver_node_type: Some("resolver_node".to_string()),
            is_blocking: true,
        };

        registry.register(schema.clone()).await;

        let fetched = registry.get_tag_schema(&"tag_1".to_string()).await.unwrap();
        assert_eq!(fetched.tag_id, schema.tag_id);
        assert_eq!(fetched.is_blocking, schema.is_blocking);
    }

    #[tokio::test]
    async fn test_get_inheritance_rules() {
        let registry = MemoryTagSchemaRegistry::new();
        let schema = TagSchema {
            tag_id: "tag_1".to_string(),
            display_name: "Test Tag".to_string(),
            expire_at: None,
            on_first_child: vec!["tag_a".to_string()],
            on_branch_child: vec!["tag_b".to_string()],
            resolver_node_type: None,
            is_blocking: false,
        };

        registry.register(schema).await;

        let first_child_rules = registry
            .get_inheritance_rules(&"tag_1".to_string(), true)
            .await
            .unwrap();
        assert_eq!(first_child_rules, vec!["tag_a".to_string()]);

        let branch_rules = registry
            .get_inheritance_rules(&"tag_1".to_string(), false)
            .await
            .unwrap();
        assert_eq!(branch_rules, vec!["tag_b".to_string()]);
    }
}
