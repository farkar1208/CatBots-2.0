//! Profile 存储实现

use crate::Profile;
use std::path::PathBuf;

/// Profile 存储接口
/// 
/// 抽象化 Profile 的持久化存储
pub trait ProfileStorage: Send + Sync {
    /// 加载所有 Profile
    fn load_all(&self) -> Result<Vec<Profile>, anyhow::Error>;
    
    /// 保存所有 Profile
    fn save_all(&self, profiles: &[Profile]) -> Result<(), anyhow::Error>;
}

/// 文件存储实现
/// 
/// 将 Profile 存储为 JSON 文件
pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    /// 创建文件存储
    /// 
    /// # 参数
    /// - `path`: 存储文件路径（如 `~/.catbots/profiles.json`）
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// 创建默认路径的文件存储
    /// 
    /// 路径: `{data_dir}/catbots/profiles.json`
    pub fn default_path() -> Result<Self, anyhow::Error> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("catbots");
        
        // 确保目录存在
        std::fs::create_dir_all(&data_dir)?;
        
        Ok(Self::new(data_dir.join("profiles.json")))
    }
}

impl ProfileStorage for FileStorage {
    fn load_all(&self) -> Result<Vec<Profile>, anyhow::Error> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.path)?;
        
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let profiles: Vec<Profile> = serde_json::from_str(&content)?;
        tracing::info!(
            path = %self.path.display(),
            count = profiles.len(),
            "已加载 Profile 文件"
        );
        
        Ok(profiles)
    }

    fn save_all(&self, profiles: &[Profile]) -> Result<(), anyhow::Error> {
        // 确保父目录存在
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(profiles)?;
        std::fs::write(&self.path, content)?;
        
        tracing::info!(
            path = %self.path.display(),
            count = profiles.len(),
            "已保存 Profile 文件"
        );
        
        Ok(())
    }
}

/// 内存存储（用于测试）
#[derive(Default)]
pub struct MemoryStorage {
    profiles: std::sync::Mutex<Vec<Profile>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProfileStorage for MemoryStorage {
    fn load_all(&self) -> Result<Vec<Profile>, anyhow::Error> {
        Ok(self.profiles.lock().unwrap().clone())
    }

    fn save_all(&self, profiles: &[Profile]) -> Result<(), anyhow::Error> {
        *self.profiles.lock().unwrap() = profiles.to_vec();
        Ok(())
    }
}
