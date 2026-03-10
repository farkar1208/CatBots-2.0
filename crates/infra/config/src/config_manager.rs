//! 配置管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM 配置
    pub llm: LLMConfig,
    /// UI 配置
    pub ui: UIConfig,
    /// 存储配置
    pub storage: StorageConfig,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// API 端点
    pub endpoint: String,
    /// API 密钥（从环境变量读取）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 温度参数
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: None,
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// UI 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    /// 主题
    #[serde(default)]
    pub theme: String,
    /// 显示行数
    #[serde(default = "default_max_display_lines")]
    pub max_display_lines: usize,
}

fn default_max_display_lines() -> usize {
    100
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            max_display_lines: 100,
        }
    }
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 数据目录
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    /// 存储后端
    #[serde(default)]
    pub backend: String,
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./data")
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            backend: "file".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig::default(),
            ui: UIConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

/// 配置管理器
pub struct ConfigManager {
    config: Config,
    config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// 创建新的配置管理器（使用默认配置）
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            config_path: None,
        }
    }

    /// 从文件加载配置
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, anyhow::Error> {
        let path = path.into();
        // TODO: 实现配置文件加载
        // 支持 TOML、JSON、YAML 格式
        Ok(Self {
            config: Config::default(),
            config_path: Some(path),
        })
    }

    /// 获取配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取可变配置
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), anyhow::Error> {
        // TODO: 实现配置保存
        todo!("实现配置保存")
    }

    /// 重新加载配置
    pub fn reload(&mut self) -> Result<(), anyhow::Error> {
        // TODO: 实现配置重新加载
        todo!("实现配置重新加载")
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
