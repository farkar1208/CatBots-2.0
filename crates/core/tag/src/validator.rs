use async_trait::async_trait;

use crate::registry::TagSchemaRegistry;
use crate::storage::TagStorage;
use crate::types::{
    InstanceId, NodeId, Result, TagError, TagId, TagInstance, TagSchema, TagStatus, ValidationResult,
};

pub struct TagValidator<S, R> {
    storage: S,
    registry: R,
}

impl<S, R> TagValidator<S, R>
where
    S: TagStorage,
    R: TagSchemaRegistry,
{
    pub fn new(storage: S, registry: R) -> Self {
        Self { storage, registry }
    }

    /// F1: 校验普通节点创建
    pub async fn validate_normal_node_creation(
        &self,
        parent_id: &NodeId,
        _node_type: &str,
    ) -> Result<ValidationResult> {
        // 1. 获取父节点所有标签
        let tags = self.storage.get_node_tags(parent_id, None).await?;

        // 2. 检查阻塞标签
        let mut blocking_tags = Vec::new();
        for tag in &tags {
            let schema = self.registry.get_tag_schema(&tag.tag_id).await?;
            if self.is_blocking(&schema, &tag).await? {
                blocking_tags.push(tag.clone());
            }
        }

        // 3. 返回结果
        if blocking_tags.is_empty() {
            Ok(ValidationResult {
                allowed: true,
                blocking_tags: vec![],
                message: "允许创建".to_string(),
            })
        } else {
            let tag_names: Vec<_> = blocking_tags.iter().map(|t| t.tag_id.clone()).collect();
            Ok(ValidationResult {
                allowed: false,
                blocking_tags,
                message: format!("存在必须处理的标签：{:?}", tag_names),
            })
        }
    }

    /// F4: 验证处理节点参数
    pub async fn validate_resolver_node(
        &self,
        parent_id: &NodeId,
        tag_instance_id: &InstanceId,
        resolver_type: &str,
    ) -> Result<TagInstance> {
        // 1. 获取标签实例
        let instance = self.storage.get_tag_instance(tag_instance_id).await?;

        // 2. 验证节点归属
        if instance.node_id != *parent_id {
            return Err(TagError::InstanceNotBelongToNode(
                tag_instance_id.clone(),
                parent_id.clone(),
            ));
        }

        // 3. 验证resolver类型匹配
        let resolver = self.registry.get_resolver_type(&instance.tag_id).await?;
        match resolver {
            Some(expected) if expected == resolver_type => Ok(instance),
            Some(expected) => Err(TagError::ResolverTypeMismatch {
                expected,
                actual: resolver_type.to_string(),
            }),
            None => Err(TagError::NoResolverDefined(instance.tag_id.clone())),
        }
    }

    /// F8: 检查标签是否阻塞（内部方法）
    pub async fn is_blocking(
        &self,
        schema: &TagSchema,
        instance: &TagInstance,
    ) -> Result<bool> {
        // 1. 检查是否已撤销
        if instance.status == TagStatus::Revoked {
            return Ok(false);
        }

        // 2. 检查is_blocking标记
        if schema.is_blocking {
            return Ok(true);
        }

        // 3. 检查是否过期
        if let Some(expire_at) = schema.expire_at {
            let now = chrono::Utc::now().timestamp();
            if now >= expire_at {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::memory::MemoryTagSchemaRegistry;
    use crate::storage::memory::MemoryTagStorage;
    use crate::types::TagSchema;

    async fn setup() -> (MemoryTagStorage, MemoryTagSchemaRegistry) {
        (MemoryTagStorage::new(), MemoryTagSchemaRegistry::new())
    }

    #[tokio::test]
    async fn test_validate_normal_node_creation_allowed() {
        let (storage, registry) = setup().await;
        let validator = TagValidator::new(storage.clone(), registry.clone());

        // 注册非阻塞标签
        registry
            .register(TagSchema {
                tag_id: "tag_1".to_string(),
                display_name: "Non-blocking".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: None,
                is_blocking: false,
            })
            .await;

        // 创建标签实例
        storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();

        let result = validator
            .validate_normal_node_creation(&"node_1".to_string(), "llm_node")
            .await
            .unwrap();

        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_validate_normal_node_creation_blocked() {
        let (storage, registry) = setup().await;
        let validator = TagValidator::new(storage.clone(), registry.clone());

        // 注册阻塞标签
        registry
            .register(TagSchema {
                tag_id: "blocking_tag".to_string(),
                display_name: "Blocking".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: Some("resolver".to_string()),
                is_blocking: true,
            })
            .await;

        // 创建阻塞标签实例
        storage
            .create_tag_instance(&"node_1".to_string(), &"blocking_tag".to_string())
            .await
            .unwrap();

        let result = validator
            .validate_normal_node_creation(&"node_1".to_string(), "llm_node")
            .await
            .unwrap();

        assert!(!result.allowed);
        assert_eq!(result.blocking_tags.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_resolver_node_success() {
        let (storage, registry) = setup().await;
        let validator = TagValidator::new(storage.clone(), registry.clone());

        // 注册标签
        registry
            .register(TagSchema {
                tag_id: "tag_1".to_string(),
                display_name: "Test".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: Some("resolver_node".to_string()),
                is_blocking: true,
            })
            .await;

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();

        // 验证成功
        let result = validator
            .validate_resolver_node(
                &"node_1".to_string(),
                &instance.instance_id,
                "resolver_node",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_resolver_node_type_mismatch() {
        let (storage, registry) = setup().await;
        let validator = TagValidator::new(storage.clone(), registry.clone());

        // 注册标签
        registry
            .register(TagSchema {
                tag_id: "tag_1".to_string(),
                display_name: "Test".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: Some("correct_resolver".to_string()),
                is_blocking: true,
            })
            .await;

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();

        // 验证失败（类型不匹配）
        let result = validator
            .validate_resolver_node(
                &"node_1".to_string(),
                &instance.instance_id,
                "wrong_resolver",
            )
            .await;

        assert!(matches!(result, Err(TagError::ResolverTypeMismatch { .. })));
    }
}
