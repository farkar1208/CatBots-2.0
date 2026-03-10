//! 持久化服务

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// 存储后端类型
#[derive(Debug, Clone, Copy)]
pub enum StorageBackend {
    /// 文件系统
    File,
    /// SQLite 数据库
    SQLite,
    /// 内存存储（用于测试）
    Memory,
}

/// 持久化服务 trait
/// 
/// 定义统一的持久化接口，支持不同的存储后端
#[async_trait]
pub trait PersistenceService: Send + Sync {
    /// 保存数据
    async fn save<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        data: &T,
    ) -> Result<(), anyhow::Error>;
    
    /// 加载数据
    async fn load<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, anyhow::Error>;
    
    /// 删除数据
    async fn delete(&self, key: &str) -> Result<(), anyhow::Error>;
    
    /// 检查数据是否存在
    async fn exists(&self, key: &str) -> Result<bool, anyhow::Error>;
}

/// 文件系统持久化服务
pub struct FilePersistence {
    /// 存储目录
    base_path: std::path::PathBuf,
}

impl FilePersistence {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

#[async_trait]
impl PersistenceService for FilePersistence {
    async fn save<T: Serialize + Send + Sync>(
        &self,
        _key: &str,
        _data: &T,
    ) -> Result<(), anyhow::Error> {
        // TODO: 实现文件保存
        todo!("实现文件保存")
    }
    
    async fn load<T: DeserializeOwned + Send + Sync>(
        &self,
        _key: &str,
    ) -> Result<Option<T>, anyhow::Error> {
        // TODO: 实现文件加载
        todo!("实现文件加载")
    }
    
    async fn delete(&self, _key: &str) -> Result<(), anyhow::Error> {
        // TODO: 实现文件删除
        todo!("实现文件删除")
    }
    
    async fn exists(&self, _key: &str) -> Result<bool, anyhow::Error> {
        // TODO: 实现文件存在检查
        todo!("实现文件存在检查")
    }
}

/// 内存持久化服务（用于测试）
pub struct MemoryPersistence {
    // TODO: 添加内部存储
    // data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryPersistence {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MemoryPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PersistenceService for MemoryPersistence {
    async fn save<T: Serialize + Send + Sync>(
        &self,
        _key: &str,
        _data: &T,
    ) -> Result<(), anyhow::Error> {
        // TODO: 实现内存保存
        todo!("实现内存保存")
    }
    
    async fn load<T: DeserializeOwned + Send + Sync>(
        &self,
        _key: &str,
    ) -> Result<Option<T>, anyhow::Error> {
        // TODO: 实现内存加载
        todo!("实现内存加载")
    }
    
    async fn delete(&self, _key: &str) -> Result<(), anyhow::Error> {
        // TODO: 实现内存删除
        todo!("实现内存删除")
    }
    
    async fn exists(&self, _key: &str) -> Result<bool, anyhow::Error> {
        // TODO: 实现内存存在检查
        todo!("实现内存存在检查")
    }
}
