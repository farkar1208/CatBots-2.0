//! Profile 模块 - 配置存储
//!
//! 核心职责：
//! - `ProfileManager`: Profile 存储与查询
//! - `Profile`: 配置档案
//! - `ModelParameters`: 模型参数
//! - `ProfileStorage`: 存储接口
//! - `FileStorage`: 文件存储实现
//! - `MemoryStorage`: 内存存储实现

mod model_parameters;
mod profile;
mod profile_manager;
mod storage;

pub use model_parameters::ModelParameters;
pub use profile::Profile;
pub use profile_manager::ProfileManager;
pub use storage::{FileStorage, MemoryStorage, ProfileStorage};