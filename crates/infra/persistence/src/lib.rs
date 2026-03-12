//! Persistence 模块 - 持久化服务
//!
//! 核心职责：
//! - 提供数据持久化接口
//! - 支持多种存储后端（文件、数据库等）

pub mod persistence_service;

pub use persistence_service::{PersistenceService, StorageBackend, MemoryPersistence, FilePersistence};
