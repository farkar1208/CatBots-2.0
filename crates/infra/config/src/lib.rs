//! Config 模块 - 配置管理
//!
//! 核心职责：
//! - 加载和管理应用配置
//! - 支持多种配置格式（TOML、JSON、YAML）

mod config_manager;

pub use config_manager::{Config, ConfigManager};
