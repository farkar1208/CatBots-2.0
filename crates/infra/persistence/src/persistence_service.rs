//! 持久化服务

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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

    /// 获取文件路径
    fn get_file_path(&self, key: &str) -> std::path::PathBuf {
        // 使用 key 作为文件名，并添加 .json 扩展名
        let safe_key = key.replace('/', "_").replace('\\', "_");
        self.base_path.join(format!("{}.json", safe_key))
    }

    /// 确保目录存在
    fn ensure_dir_exists(&self) -> Result<(), anyhow::Error> {
        std::fs::create_dir_all(&self.base_path)?;
        Ok(())
    }
}

#[async_trait]
impl PersistenceService for FilePersistence {
    async fn save<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        data: &T,
    ) -> Result<(), anyhow::Error> {
        let file_path = self.get_file_path(key);
        
        // 确保目录存在
        self.ensure_dir_exists()?;

        // 序列化数据
        let content = serde_json::to_string_pretty(data)?;

        // 异步写入文件（使用 tokio 的 fs 模块）
        tokio::fs::write(&file_path, content).await?;

        tracing::debug!(
            key = %key,
            path = %file_path.display(),
            "数据已保存"
        );

        Ok(())
    }
    
    async fn load<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, anyhow::Error> {
        let file_path = self.get_file_path(key);

        // 检查文件是否存在
        if !tokio::fs::try_exists(&file_path).await? {
            return Ok(None);
        }

        // 读取文件内容
        let content = tokio::fs::read_to_string(&file_path).await?;

        // 反序列化
        let data = serde_json::from_str(&content)?;

        tracing::debug!(
            key = %key,
            path = %file_path.display(),
            "数据已加载"
        );

        Ok(Some(data))
    }
    
    async fn delete(&self, key: &str) -> Result<(), anyhow::Error> {
        let file_path = self.get_file_path(key);

        // 检查文件是否存在
        if tokio::fs::try_exists(&file_path).await? {
            tokio::fs::remove_file(&file_path).await?;
            tracing::debug!(
                key = %key,
                path = %file_path.display(),
                "数据已删除"
            );
        }

        Ok(())
    }
    
    async fn exists(&self, key: &str) -> Result<bool, anyhow::Error> {
        let file_path = self.get_file_path(key);
        Ok(tokio::fs::try_exists(&file_path).await?)
    }
}

/// 内存持久化服务（用于测试）
#[derive(Clone)]
pub struct MemoryPersistence {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryPersistence {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取数据引用（用于测试）
    pub async fn get_data(&self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.lock().await;
        data.get(key).cloned()
    }

    /// 清空所有数据（用于测试）
    pub async fn clear(&self) {
        let mut data = self.data.lock().await;
        data.clear();
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
        key: &str,
        data: &T,
    ) -> Result<(), anyhow::Error> {
        let bytes = serde_json::to_vec(data)?;
        let mut storage = self.data.lock().await;
        storage.insert(key.to_string(), bytes);
        tracing::debug!(key = %key, "数据已保存到内存");
        Ok(())
    }
    
    async fn load<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, anyhow::Error> {
        let storage = self.data.lock().await;
        if let Some(bytes) = storage.get(key) {
            let data: T = serde_json::from_slice(bytes)?;
            tracing::debug!(key = %key, "数据已从内存加载");
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<(), anyhow::Error> {
        let mut storage = self.data.lock().await;
        storage.remove(key);
        tracing::debug!(key = %key, "数据已从内存删除");
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> Result<bool, anyhow::Error> {
        let storage = self.data.lock().await;
        Ok(storage.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: String,
        count: i32,
    }

    #[tokio::test]
    async fn test_memory_persistence_save_load() {
        let persistence = MemoryPersistence::new();
        let key = "test_key";
        let data = "test_value".to_string();

        // 保存
        persistence.save(key, &data).await.unwrap();

        // 加载
        let loaded: Option<String> = persistence.load(key).await.unwrap();
        assert_eq!(loaded, Some(data));

        // 检查存在
        assert!(persistence.exists(key).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_persistence_delete() {
        let persistence = MemoryPersistence::new();
        let key = "test_key_delete";
        let data = "test_value".to_string();

        // 保存
        persistence.save(key, &data).await.unwrap();
        assert!(persistence.exists(key).await.unwrap());

        // 删除
        persistence.delete(key).await.unwrap();
        assert!(!persistence.exists(key).await.unwrap());

        // 加载应该返回 None
        let loaded: Option<String> = persistence.load(key).await.unwrap();
        assert_eq!(loaded, None);
    }

    #[tokio::test]
    async fn test_memory_persistence_clear() {
        let persistence = MemoryPersistence::new();

        persistence.save("key1", &"value1").await.unwrap();
        persistence.save("key2", &"value2").await.unwrap();

        assert!(persistence.exists("key1").await.unwrap());
        assert!(persistence.exists("key2").await.unwrap());

        persistence.clear().await;

        assert!(!persistence.exists("key1").await.unwrap());
        assert!(!persistence.exists("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_persistence_complex_type() {
        let persistence = MemoryPersistence::new();
        let data = TestData {
            id: "test".to_string(),
            count: 42,
        };

        persistence.save("complex", &data).await.unwrap();

        let loaded: Option<TestData> = persistence.load("complex").await.unwrap();
        assert_eq!(loaded, Some(data));
    }
}
