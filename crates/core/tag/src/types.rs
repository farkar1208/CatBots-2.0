use serde::{Deserialize, Serialize};

// 标识类型
pub type TagId = String;
pub type InstanceId = String;
pub type NodeId = String;

// 标签状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagStatus {
    Active,
    Revoked,
}

// 校验结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub allowed: bool,
    pub blocking_tags: Vec<TagInstance>,
    pub message: String,
}

// 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub message: String,
}

// 标签定义（Schema）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSchema {
    pub tag_id: TagId,
    pub display_name: String,
    pub expire_at: Option<i64>, // Unix timestamp, None表示永不过期
    pub on_first_child: Vec<TagId>,
    pub on_branch_child: Vec<TagId>,
    pub resolver_node_type: Option<String>, // None表示无法通过节点处理
    pub is_blocking: bool,
}

// 标签实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInstance {
    pub instance_id: InstanceId,
    pub node_id: NodeId,
    pub tag_id: TagId,
    pub created_time: i64,
    pub status: TagStatus,
}

// 继承的上下文信息
#[derive(Debug, Clone)]
pub struct InheritanceContext {
    pub parent_id: NodeId,
    pub child_index: usize, // 0表示长子
    pub is_first_child: bool,
}

// 创建节点结果
#[derive(Debug, Clone)]
pub struct CreateNodeResult {
    pub node_id: NodeId,
    pub inherited_tags: Vec<TagInstance>,
}

// 创建处理节点结果
#[derive(Debug, Clone)]
pub struct CreateResolverResult {
    pub node_id: NodeId,
    pub processed_instance_id: InstanceId,
    pub copied_tags: Vec<TagInstance>,
    pub process_result: ProcessResult,
}

// 错误类型
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Blocked by tags: {0:?}")]
    BlockedByTags(Vec<TagInstance>),

    #[error("Resolver type mismatch: expected {expected}, got {actual}")]
    ResolverTypeMismatch { expected: String, actual: String },

    #[error("No resolver defined for tag: {0}")]
    NoResolverDefined(TagId),

    #[error("Instance {0} does not belong to node {1}")]
    InstanceNotBelongToNode(InstanceId, NodeId),

    #[error("Already revoked: {0}")]
    AlreadyRevoked(InstanceId),

    #[error("Inheritance loop detected")]
    InheritanceLoop,

    #[error("Resolver not found: {0}")]
    ResolverNotFound(TagId),

    #[error("Operator execution failed: {0}")]
    OperatorExecutionFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Audit error: {0}")]
    AuditError(String),

    #[error("Node creation failed: {0}")]
    NodeCreationFailed(String),
}

pub type Result<T> = std::result::Result<T, TagError>;
