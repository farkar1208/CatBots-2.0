//! Profile 管理器

use crate::{Profile, ProfileStorage};
use std::collections::HashMap;

/// Profile 管理器
/// 
/// 核心职责：
/// - Profile 存储与查询
/// - 管理多个配置档案
pub struct ProfileManager {
    /// Profile 映射表
    profiles: HashMap<String, Profile>,
    /// 存储后端
    storage: Box<dyn ProfileStorage>,
}

impl ProfileManager {
    /// 创建新的 Profile 管理器
    pub fn new(storage: Box<dyn ProfileStorage>) -> Result<Self, anyhow::Error> {
        let profiles = storage.load_all()?;
        let map: HashMap<String, Profile> = profiles
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();
        
        Ok(Self { profiles: map, storage })
    }

    /// 创建使用内存存储的管理器（用于测试）
    pub fn with_memory_storage() -> Self {
        Self {
            profiles: HashMap::new(),
            storage: Box::new(crate::MemoryStorage::new()),
        }
    }

    /// 创建使用默认文件存储的管理器
    /// 
    /// 文件路径: `{data_dir}/catbots/profiles.json`
    pub fn with_file_storage() -> Result<Self, anyhow::Error> {
        let storage = crate::FileStorage::default_path()?;
        Self::new(Box::new(storage))
    }

    /// 创建使用指定路径的文件存储的管理器
    pub fn with_file_storage_at(path: std::path::PathBuf) -> Result<Self, anyhow::Error> {
        let storage = crate::FileStorage::new(path);
        Self::new(Box::new(storage))
    }

    /// 获取指定 Profile
    pub fn get(&self, profile_id: &str) -> Option<&Profile> {
        self.profiles.get(profile_id)
    }

    /// 列出所有 Profile
    pub fn list(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }

    /// 添加 Profile（自动持久化）
    pub fn add(&mut self, profile: Profile) -> Result<(), anyhow::Error> {
        if self.profiles.contains_key(&profile.id) {
            return Err(anyhow::anyhow!("Profile '{}' already exists", profile.id));
        }
        self.profiles.insert(profile.id.clone(), profile);
        self.persist()
    }

    /// 移除 Profile（自动持久化）
    pub fn remove(&mut self, profile_id: &str) -> Result<(), anyhow::Error> {
        if self.profiles.remove(profile_id).is_none() {
            return Err(anyhow::anyhow!("Profile '{}' not found", profile_id));
        }
        self.persist()
    }

    /// 更新 Profile（自动持久化）
    pub fn update(&mut self, profile: Profile) -> Result<(), anyhow::Error> {
        if !self.profiles.contains_key(&profile.id) {
            return Err(anyhow::anyhow!("Profile '{}' not found", profile.id));
        }
        self.profiles.insert(profile.id.clone(), profile);
        self.persist()
    }

    /// 获取默认 Profile
    pub fn get_default(&self) -> Option<&Profile> {
        self.profiles.values().find(|p| p.is_default)
    }

    /// 持久化到存储
    fn persist(&self) -> Result<(), anyhow::Error> {
        let profiles: Vec<_> = self.profiles.values().cloned().collect();
        self.storage.save_all(&profiles)
    }
}