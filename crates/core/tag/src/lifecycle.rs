use crate::storage::TagStorage;
use crate::types::{InstanceId, Result, TagError, TagSchema, TagInstance, TagStatus};

pub struct LifecycleController<S> {
    storage: S,
}

impl<S: TagStorage> LifecycleController<S> {
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// F7: 撤销标签实例
    pub async fn revoke_tag_instance(
        &self,
        instance_id: &InstanceId,
        _reason: &str,
    ) -> Result<()> {
        let instance = self.storage.get_tag_instance(instance_id).await?;

        // 检查当前状态
        if instance.status == TagStatus::Revoked {
            return Err(TagError::AlreadyRevoked(instance_id.clone()));
        }

        self.storage
            .update_tag_status(instance_id, TagStatus::Revoked)
            .await?;
        Ok(())
    }

    /// F8: 检查标签过期（对外暴露，供查询使用）
    pub async fn is_tag_expired(
        &self,
        schema: &TagSchema,
        instance: &TagInstance,
    ) -> Result<bool> {
        if instance.status == TagStatus::Revoked {
            return Ok(false);
        }

        if let Some(expire_at) = schema.expire_at {
            let now = chrono::Utc::now().timestamp();
            return Ok(now >= expire_at);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryTagStorage;
    use crate::types::TagSchema;

    #[tokio::test]
    async fn test_revoke_tag_instance() {
        let storage = MemoryTagStorage::new();
        let controller = LifecycleController::new(storage.clone());

        // 创建标签实例
        let instance = storage
            .create_tag_instance(&"node_1".to_string(), &"tag_1".to_string())
            .await
            .unwrap();

        // 撤销
        controller
            .revoke_tag_instance(&instance.instance_id, "test")
            .await
            .unwrap();

        // 验证状态
        let tags = storage
            .get_node_tags(&"node_1".to_string(), Some(TagStatus::Active))
            .await
            .unwrap();
        assert_eq!(tags.len(), 0);

        // 重复撤销应该失败
        let result = controller
            .revoke_tag_instance(&instance.instance_id, "test")
            .await;
        assert!(matches!(result, Err(TagError::AlreadyRevoked(_))));
    }

    #[tokio::test]
    async fn test_is_tag_expired() {
        let storage = MemoryTagStorage::new();
        let controller = LifecycleController::new(storage);

        // 创建已过期的schema
        let expired_schema = TagSchema {
            tag_id: "expired".to_string(),
            display_name: "Expired".to_string(),
            expire_at: Some(0), // 1970年已过期
            on_first_child: vec![],
            on_branch_child: vec![],
            resolver_node_type: None,
            is_blocking: false,
        };

        let instance = crate::types::TagInstance {
            instance_id: "inst_1".to_string(),
            node_id: "node_1".to_string(),
            tag_id: "expired".to_string(),
            created_time: 0,
            status: TagStatus::Active,
        };

        let is_expired = controller.is_tag_expired(&expired_schema, &instance).await.unwrap();
        assert!(is_expired);

        // 未过期的schema
        let valid_schema = TagSchema {
            tag_id: "valid".to_string(),
            display_name: "Valid".to_string(),
            expire_at: Some(chrono::Utc::now().timestamp() + 3600), // 1小时后过期
            on_first_child: vec![],
            on_branch_child: vec![],
            resolver_node_type: None,
            is_blocking: false,
        };

        let is_expired = controller.is_tag_expired(&valid_schema, &instance).await.unwrap();
        assert!(!is_expired);
    }
}
