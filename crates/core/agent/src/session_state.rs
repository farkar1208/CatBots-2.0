//! 会话状态 - 维护 currentNode + currentProfileId

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 会话状态
/// 
/// 维护当前会话的状态信息，包括 currentNode 指针和当前 Profile ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// 当前节点ID
    pub current_node_id: String,
    /// 当前 Profile ID
    pub current_profile_id: String,
    /// 会话ID
    pub session_id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl SessionState {
    /// 创建新的会话状态
    pub fn new() -> Self {
        Self {
            current_node_id: "root".to_string(),
            current_profile_id: "default".to_string(),
            session_id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
        }
    }

    /// 设置当前节点
    pub fn set_current_node(&mut self, node_id: String) {
        self.current_node_id = node_id;
    }

    /// 获取当前节点ID
    pub fn current_node(&self) -> &str {
        &self.current_node_id
    }

    /// 设置当前 Profile
    pub fn set_current_profile(&mut self, profile_id: String) {
        self.current_profile_id = profile_id;
    }

    /// 获取当前 Profile ID
    pub fn current_profile(&self) -> &str {
        &self.current_profile_id
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}
