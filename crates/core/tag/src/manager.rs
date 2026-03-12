use crate::audit::AuditLogger;
use crate::inheritance::{NodeStorage, TagInheritance};
use crate::operator::TagOperator;
use crate::registry::TagSchemaRegistry;
use crate::storage::TagStorage;
use crate::types::{
    CreateNodeResult, CreateResolverResult, InstanceId, NodeId, ProcessResult, Result, TagError,
    TagStatus, ValidationResult,
};
use crate::validator::TagValidator;
use crate::lifecycle::LifecycleController;

/// 节点创建者trait（外部依赖）
#[async_trait::async_trait]
pub trait NodeCreator: Send + Sync {
    async fn create_node(&self, parent_id: &NodeId) -> Result<NodeId>;
    async fn create_resolver_node(&self, parent_id: &NodeId, resolver_type: &str) -> Result<NodeId>;
}

/// TagManager - 统一入口
pub struct TagManager<S, R, N, A> {
    validator: TagValidator<S, R>,
    inheritance: TagInheritance<S, R, N>,
    operator: TagOperator<S>,
    lifecycle: LifecycleController<S>,
    audit: A,
}

impl<S, R, N, A> TagManager<S, R, N, A>
where
    S: TagStorage + Clone,
    R: TagSchemaRegistry + Clone,
    N: NodeStorage + Clone,
    A: AuditLogger + Clone,
{
    pub fn new(
        storage: S,
        registry: R,
        node_storage: N,
        audit: A,
    ) -> Self {
        let validator = TagValidator::new(storage.clone(), registry.clone());
        let inheritance = TagInheritance::new(storage.clone(), registry.clone(), node_storage);
        let operator = TagOperator::new(storage.clone());
        let lifecycle = LifecycleController::new(storage);

        Self {
            validator,
            inheritance,
            operator,
            lifecycle,
            audit,
        }
    }

    /// 获取operator的可变引用（用于注册处理算子）
    pub fn operator(&self) -> &TagOperator<S> {
        &self.operator
    }

    /// 创建普通子节点（完整流程）
    pub async fn create_normal_child_node(
        &self,
        parent_id: &NodeId,
        node_type: &str,
        node_creator: &dyn NodeCreator,
    ) -> Result<CreateNodeResult> {
        // 1. 校验
        let validation = self
            .validator
            .validate_normal_node_creation(parent_id, node_type)
            .await?;
        self.audit
            .log_validation(parent_id, node_type, &validation)
            .await?;

        if !validation.allowed {
            return Err(TagError::BlockedByTags(validation.blocking_tags));
        }

        // 2. 创建节点
        let child_id = node_creator.create_node(parent_id).await?;

        // 3. 继承标签（长子index=0，其他按当前子节点数确定）
        let child_index = 0; // 简化处理，实际应该从node_creator获取
        let inherited = self
            .inheritance
            .create_inherited_tags(parent_id, &child_id, child_index)
            .await?;
        self.audit
            .log_inheritance(parent_id, &child_id, &inherited)
            .await?;

        Ok(CreateNodeResult {
            node_id: child_id,
            inherited_tags: inherited,
        })
    }

    /// 创建处理节点（完整流程）
    pub async fn create_resolver_node(
        &self,
        parent_id: &NodeId,
        resolver_type: &str,
        tag_instance_id: &InstanceId,
        input_data: &serde_json::Value,
        node_creator: &dyn NodeCreator,
    ) -> Result<CreateResolverResult> {
        // 1. 验证
        let tag_instance = self
            .validator
            .validate_resolver_node(parent_id, tag_instance_id, resolver_type)
            .await?;

        // 2. 创建节点
        let child_id = node_creator
            .create_resolver_node(parent_id, resolver_type)
            .await?;

        // 3. 复制其他标签
        let copied = self
            .inheritance
            .copy_non_processed_tags(parent_id, &child_id, tag_instance_id)
            .await?;

        // 4. 执行处理算子
        let process_result = self.operator.execute_resolver(&tag_instance, input_data).await?;
        self.audit
            .log_tag_processed(parent_id, &child_id, tag_instance_id, &process_result)
            .await?;

        Ok(CreateResolverResult {
            node_id: child_id,
            processed_instance_id: tag_instance_id.clone(),
            copied_tags: copied,
            process_result,
        })
    }

    /// 撤销标签
    pub async fn revoke_tag(
        &self,
        instance_id: &InstanceId,
        reason: &str,
    ) -> Result<()> {
        self.lifecycle.revoke_tag_instance(instance_id, reason).await?;
        self.audit
            .log_tag_change(instance_id, TagStatus::Active, TagStatus::Revoked, reason)
            .await?;
        Ok(())
    }

    /// 校验节点创建（仅校验，不创建）
    pub async fn validate_creation(
        &self,
        parent_id: &NodeId,
        node_type: &str,
    ) -> Result<ValidationResult> {
        let result = self.validator.validate_normal_node_creation(parent_id, node_type).await?;
        self.audit.log_validation(parent_id, node_type, &result).await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::memory::MemoryAuditLogger;
    use crate::inheritance::memory::MemoryNodeStorage;
    use crate::operator::ResolverOperator;
    use crate::registry::memory::MemoryTagSchemaRegistry;
    use crate::storage::memory::MemoryTagStorage;
    use crate::types::{TagInstance, TagSchema, TagStatus};
    use async_trait::async_trait;

    struct MockNodeCreator;

    #[async_trait]
    impl NodeCreator for MockNodeCreator {
        async fn create_node(&self, parent_id: &NodeId) -> Result<NodeId> {
            Ok(format!("{}_child", parent_id))
        }

        async fn create_resolver_node(&self, parent_id: &NodeId, resolver_type: &str) -> Result<NodeId> {
            Ok(format!("{}_{}_child", parent_id, resolver_type))
        }
    }

    struct MockResolver;

    #[async_trait]
    impl ResolverOperator for MockResolver {
        async fn execute(
            &self,
            _tag_instance: &TagInstance,
            _input_data: &serde_json::Value,
        ) -> Result<ProcessResult> {
            Ok(ProcessResult {
                success: true,
                output: None,
                message: "Processed".to_string(),
            })
        }
    }

    async fn setup_manager() -> (
        TagManager<MemoryTagStorage, MemoryTagSchemaRegistry, MemoryNodeStorage, MemoryAuditLogger>,
        MemoryTagStorage,
        MemoryTagSchemaRegistry,
        MemoryNodeStorage,
    ) {
        let storage = MemoryTagStorage::new();
        let registry = MemoryTagSchemaRegistry::new();
        let node_storage = MemoryNodeStorage::new();
        let audit = MemoryAuditLogger::new();

        let manager = TagManager::new(
            storage.clone(),
            registry.clone(),
            node_storage.clone(),
            audit,
        );

        (manager, storage, registry, node_storage)
    }

    #[tokio::test]
    async fn test_create_normal_child_node_allowed() {
        let (manager, _storage, registry, _node_storage) = setup_manager().await;

        // 注册非阻塞标签
        registry
            .register(TagSchema {
                tag_id: "tag_1".to_string(),
                display_name: "Non-blocking".to_string(),
                expire_at: None,
                on_first_child: vec!["inherited".to_string()],
                on_branch_child: vec![],
                resolver_node_type: None,
                is_blocking: false,
            })
            .await;

        let node_creator = MockNodeCreator;
        let result = manager
            .create_normal_child_node(&"node_1".to_string(), "llm_node", &node_creator)
            .await;

        assert!(result.is_ok());
        let create_result = result.unwrap();
        assert_eq!(create_result.node_id, "node_1_child");
    }

    #[tokio::test]
    async fn test_create_normal_child_node_blocked() {
        let (manager, storage, registry, _node_storage) = setup_manager().await;

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

        // 创建阻塞标签实例
        storage
            .create_tag_instance(&"node_1".to_string(), &"blocking".to_string())
            .await
            .unwrap();

        let node_creator = MockNodeCreator;
        let result = manager
            .create_normal_child_node(&"node_1".to_string(), "llm_node", &node_creator)
            .await;

        assert!(matches!(result, Err(TagError::BlockedByTags(_))));
    }

    #[tokio::test]
    async fn test_create_resolver_node() {
        let (manager, storage, registry, _node_storage) = setup_manager().await;

        // 注册标签和算子
        registry
            .register(TagSchema {
                tag_id: "to_process".to_string(),
                display_name: "To Process".to_string(),
                expire_at: None,
                on_first_child: vec![],
                on_branch_child: vec![],
                resolver_node_type: Some("my_resolver".to_string()),
                is_blocking: true,
            })
            .await;

        manager
            .operator()
            .register_resolver("to_process".to_string(), Box::new(MockResolver))
            .await;

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&"node_1".to_string(), &"to_process".to_string())
            .await
            .unwrap();

        let node_creator = MockNodeCreator;
        let result = manager
            .create_resolver_node(
                &"node_1".to_string(),
                "my_resolver",
                &instance.instance_id,
                &serde_json::json!({}),
                &node_creator,
            )
            .await;

        assert!(result.is_ok());
        let resolver_result = result.unwrap();
        assert_eq!(resolver_result.processed_instance_id, instance.instance_id);
        assert!(resolver_result.process_result.success);
    }

    #[tokio::test]
    async fn test_revoke_tag() {
        let (manager, storage, _registry, _node_storage) = setup_manager().await;

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();

        // 撤销
        let result = manager
            .revoke_tag(&instance.instance_id, "test revoke")
            .await;
        assert!(result.is_ok());

        // 验证状态
        let tags = storage
            .get_node_tags(&"node_1".to_string(), Some(TagStatus::Active))
            .await
            .unwrap();
        assert_eq!(tags.len(), 0);
    }
}
