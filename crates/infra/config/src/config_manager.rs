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

        // 如果文件不存在，使用默认配置并设置路径
        if !path.exists() {
            return Ok(Self {
                config: Config::default(),
                config_path: Some(path),
            });
        }

        // 读取文件内容
        let content = std::fs::read_to_string(&path)?;

        // 解析 JSON
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("配置文件解析失败: {}", e))?;

        tracing::info!(
            path = %path.display(),
            "已从文件加载配置"
        );

        Ok(Self {
            config,
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
        if let Some(ref path) = self.config_path {
            // 确保父目录存在
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // 序列化为 JSON
            let content = serde_json::to_string_pretty(&self.config)?;

            // 写入文件
            std::fs::write(path, content)?;

            tracing::info!(
                path = %path.display(),
                "配置已保存"
            );

            Ok(())
        } else {
            Err(anyhow::anyhow!("未设置配置文件路径，无法保存"))
        }
    }

    /// 重新加载配置
    pub fn reload(&mut self) -> Result<(), anyhow::Error> {
        if let Some(ref path) = self.config_path {
            // 如果文件不存在，使用默认配置
            if !path.exists() {
                self.config = Config::default();
                tracing::warn!(
                    path = %path.display(),
                    "配置文件不存在，使用默认配置"
                );
                return Ok(());
            }

            // 读取并解析文件
            let content = std::fs::read_to_string(path)?;
            self.config = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("配置文件解析失败: {}", e))?;

            tracing::info!(
                path = %path.display(),
                "配置已重新加载"
            );

            Ok(())
        } else {
            Err(anyhow::anyhow!("未设置配置文件路径，无法重新加载"))
        }
    }

    /// 设置配置文件路径
    pub fn set_config_path(&mut self, path: impl Into<PathBuf>) {
        self.config_path = Some(path.into());
    }

    /// 获取配置文件路径
    pub fn config_path(&self) -> Option<&PathBuf> {
        self.config_path.as_ref()
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let manager = ConfigManager::new();
        let config = manager.config();

        assert_eq!(config.llm.endpoint, "https://api.openai.com/v1");
        assert_eq!(config.llm.model, "gpt-4");
        assert_eq!(config.llm.temperature, 0.7);
        assert_eq!(config.llm.max_tokens, 4096);
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // 创建配置管理器并修改配置
        let mut manager = ConfigManager::new();
        manager.set_config_path(&config_path);
        manager.config_mut().llm.model = "gpt-4o".to_string();
        manager.config_mut().llm.temperature = 0.8;

        // 保存配置
        manager.save().unwrap();

        // 从文件加载
        let loaded_manager = ConfigManager::from_file(&config_path).unwrap();
        assert_eq!(loaded_manager.config().llm.model, "gpt-4o");
        assert_eq!(loaded_manager.config().llm.temperature, 0.8);
    }

    #[test]
    fn test_config_reload() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // 创建并保存配置
        let mut manager = ConfigManager::new();
        manager.set_config_path(&config_path);
        manager.config_mut().llm.model = "gpt-4o".to_string();
        manager.save().unwrap();

        // 手动修改文件
        let content = r#"{
            "llm": {
                "endpoint": "https://api.openai.com/v1",
                "model": "claude-sonnet-4",
                "max_tokens": 4096,
                "temperature": 0.5
            },
            "ui": {
                "theme": "dark",
                "max_display_lines": 100
            },
            "storage": {
                "data_dir": "./data",
                "backend": "file"
            }
        }"#;
        std::fs::write(&config_path, content).unwrap();

        // 重新加载
        manager.reload().unwrap();
        assert_eq!(manager.config().llm.model, "claude-sonnet-4");
        assert_eq!(manager.config().llm.temperature, 0.5);
    }

    #[test]
    fn test_config_from_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.json");

        // 从不存在的文件加载应该返回默认配置
        let manager = ConfigManager::from_file(&config_path).unwrap();
        assert_eq!(manager.config().llm.model, "gpt-4");
        assert_eq!(manager.config_path(), Some(&config_path));
    }

    #[test]
    fn test_config_save_without_path() {
        let manager = ConfigManager::new();
        let result = manager.save();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未设置配置文件路径"));
    }

    #[test]
    fn test_config_reload_without_path() {
        let mut manager = ConfigManager::new();
        let result = manager.reload();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未设置配置文件路径"));
    }
}
