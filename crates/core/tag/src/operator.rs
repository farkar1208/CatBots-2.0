use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::storage::TagStorage;
use crate::types::{ProcessResult, Result, TagError, TagId, TagInstance};

/// 处理算子trait
#[async_trait]
pub trait ResolverOperator: Send + Sync {
    async fn execute(
        &self,
        tag_instance: &TagInstance,
        input_data: &serde_json::Value,
    ) -> Result<ProcessResult>;
}

pub struct TagOperator<S> {
    storage: S,
    resolvers: Arc<RwLock<HashMap<TagId, Box<dyn ResolverOperator>>>>,
}

impl<S: TagStorage> TagOperator<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            resolvers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// F6: 执行处理算子
    pub async fn execute_resolver(
        &self,
        tag_instance: &TagInstance,
        input_data: &serde_json::Value,
    ) -> Result<ProcessResult> {
        let registry = self.resolvers.read().await;
        let resolver = registry
            .get(&tag_instance.tag_id)
            .ok_or_else(|| TagError::ResolverNotFound(tag_instance.tag_id.clone()))?;

        resolver.execute(tag_instance, input_data).await
    }

    /// 注册处理算子
    pub async fn register_resolver(
        &self,
        tag_id: TagId,
        resolver: Box<dyn ResolverOperator>,
    ) {
        let mut registry = self.resolvers.write().await;
        registry.insert(tag_id, resolver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryTagStorage;

    struct MockResolver;

    #[async_trait]
    impl ResolverOperator for MockResolver {
        async fn execute(
            &self,
            _tag_instance: &TagInstance,
            input_data: &serde_json::Value,
        ) -> Result<ProcessResult> {
            Ok(ProcessResult {
                success: true,
                output: Some(input_data.clone()),
                message: "Mock execution completed".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_execute_resolver() {
        let storage = MemoryTagStorage::new();
        let operator = TagOperator::new(storage);

        // 注册算子
        operator
            .register_resolver("test_tag".to_string(), Box::new(MockResolver))
            .await;

        // 创建标签实例
        let instance = TagInstance {
            instance_id: "inst_1".to_string(),
            node_id: "node_1".to_string(),
            tag_id: "test_tag".to_string(),
            created_time: 0,
            status: crate::types::TagStatus::Active,
        };

        // 执行
        let input = serde_json::json!({"key": "value"});
        let result = operator.execute_resolver(&instance, &input).await.unwrap();

        assert!(result.success);
        assert_eq!(result.output, Some(input));
    }

    #[tokio::test]
    async fn test_resolver_not_found() {
        let storage = MemoryTagStorage::new();
        let operator = TagOperator::new(storage);

        let instance = TagInstance {
            instance_id: "inst_1".to_string(),
            node_id: "node_1".to_string(),
            tag_id: "unknown_tag".to_string(),
            created_time: 0,
            status: crate::types::TagStatus::Active,
        };

        let result = operator
            .execute_resolver(&instance, &serde_json::json!({}))
            .await;

        assert!(matches!(result, Err(TagError::ResolverNotFound(_))));
    }
}
