use async_trait::async_trait;

use crate::types::{InstanceId, NodeId, ProcessResult, Result, TagError, TagInstance, TagStatus, ValidationResult};

/// 审计日志trait
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// A1: 记录校验日志
    async fn log_validation(
        &self,
        parent_id: &NodeId,
        node_type: &str,
        result: &ValidationResult,
    ) -> Result<()>;

    /// A2: 记录继承日志
    async fn log_inheritance(
        &self,
        parent_id: &NodeId,
        child_id: &NodeId,
        inherited_tags: &[TagInstance],
    ) -> Result<()>;

    /// A3: 记录处理日志
    async fn log_tag_processed(
        &self,
        parent_id: &NodeId,
        child_id: &NodeId,
        tag_instance_id: &InstanceId,
        result: &ProcessResult,
    ) -> Result<()>;

    /// A4: 记录状态变更日志
    async fn log_tag_change(
        &self,
        instance_id: &InstanceId,
        old_status: TagStatus,
        new_status: TagStatus,
        reason: &str,
    ) -> Result<()>;
}

/// 内存实现（用于测试，仅打印日志）
pub mod memory {
    use super::*;

    #[derive(Clone)]
    pub struct MemoryAuditLogger;

    impl MemoryAuditLogger {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl AuditLogger for MemoryAuditLogger {
        async fn log_validation(
            &self,
            parent_id: &NodeId,
            node_type: &str,
            result: &ValidationResult,
        ) -> Result<()> {
            println!(
                "[AUDIT] Validation: parent={}, node_type={}, allowed={}, message={}",
                parent_id, node_type, result.allowed, result.message
            );
            Ok(())
        }

        async fn log_inheritance(
            &self,
            parent_id: &NodeId,
            child_id: &NodeId,
            inherited_tags: &[TagInstance],
        ) -> Result<()> {
            let tag_ids: Vec<_> = inherited_tags.iter().map(|t| &t.tag_id).collect();
            println!(
                "[AUDIT] Inheritance: parent={} -> child={}, tags={:?}",
                parent_id, child_id, tag_ids
            );
            Ok(())
        }

        async fn log_tag_processed(
            &self,
            parent_id: &NodeId,
            child_id: &NodeId,
            tag_instance_id: &InstanceId,
            result: &ProcessResult,
        ) -> Result<()> {
            println!(
                "[AUDIT] Tag processed: parent={} -> child={}, instance={}, success={}",
                parent_id, child_id, tag_instance_id, result.success
            );
            Ok(())
        }

        async fn log_tag_change(
            &self,
            instance_id: &InstanceId,
            old_status: TagStatus,
            new_status: TagStatus,
            reason: &str,
        ) -> Result<()> {
            println!(
                "[AUDIT] Status change: instance={} {:?} -> {:?}, reason={}",
                instance_id, old_status, new_status, reason
            );
            Ok(())
        }
    }
}

/// 空实现（生产环境可以替换为真实日志系统）
pub struct NoOpAuditLogger;

impl NoOpAuditLogger {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AuditLogger for NoOpAuditLogger {
    async fn log_validation(
        &self,
        _parent_id: &NodeId,
        _node_type: &str,
        _result: &ValidationResult,
    ) -> Result<()> {
        Ok(())
    }

    async fn log_inheritance(
        &self,
        _parent_id: &NodeId,
        _child_id: &NodeId,
        _inherited_tags: &[TagInstance],
    ) -> Result<()> {
        Ok(())
    }

    async fn log_tag_processed(
        &self,
        _parent_id: &NodeId,
        _child_id: &NodeId,
        _tag_instance_id: &InstanceId,
        _result: &ProcessResult,
    ) -> Result<()> {
        Ok(())
    }

    async fn log_tag_change(
        &self,
        _instance_id: &InstanceId,
        _old_status: TagStatus,
        _new_status: TagStatus,
        _reason: &str,
    ) -> Result<()> {
        Ok(())
    }
}
