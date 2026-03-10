//! Agent 模块 - 会话状态管理
//!
//! 核心职责：
//! - `SessionAgent`: 协调各模块，处理用户输入
//! - `SessionState`: 维护 `currentNode` 指针

mod session_agent;
mod session_state;

pub use session_agent::SessionAgent;
pub use session_state::SessionState;
