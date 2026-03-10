//! 应用程序核心逻辑
//!
//! 负责组装各个模块并协调运行

use catbots_agent::SessionAgent;
use catbots_config::ConfigManager;
use catbots_history::ConversationTree;
use catbots_profile::ProfileManager;
use std::sync::{Arc, Mutex};

/// 应用程序
/// 
/// 持有各个核心组件的实例
pub struct App {
    /// 会话代理（核心协调器）
    agent: SessionAgent,
    /// 配置管理
    config: ConfigManager,
}

impl App {
    /// 创建新的应用实例
    pub fn new() -> Self {
        // 创建对话树（单例，使用 Arc<Mutex<>> 共享）
        let tree = Arc::new(Mutex::new(ConversationTree::new()));
        
        // 创建 Profile 管理器
        let profile_manager = ProfileManager::with_memory_storage();
        
        // 创建会话代理，传入共享的对话树
        let agent = SessionAgent::with_tree(profile_manager, tree);
        
        Self {
            agent,
            config: ConfigManager::new(),
        }
    }

    /// 使用配置创建应用实例
    pub fn with_config(config: ConfigManager) -> Self {
        let tree = Arc::new(Mutex::new(ConversationTree::new()));
        let profile_manager = ProfileManager::with_memory_storage();
        let agent = SessionAgent::with_tree(profile_manager, tree);
        
        Self { agent, config }
    }

    /// 获取会话代理
    pub fn agent(&self) -> &SessionAgent {
        &self.agent
    }

    /// 获取可变会话代理
    pub fn agent_mut(&mut self) -> &mut SessionAgent {
        &mut self.agent
    }

    /// 获取对话树引用
    pub fn tree(&self) -> Arc<Mutex<ConversationTree>> {
        self.agent.tree()
    }

    /// 获取配置
    pub fn config(&self) -> &ConfigManager {
        &self.config
    }

    /// 获取 Profile 管理器
    pub fn profile_manager(&self) -> &ProfileManager {
        self.agent.profile_manager()
    }

    /// 获取可变 Profile 管理器
    pub fn profile_manager_mut(&mut self) -> &mut ProfileManager {
        self.agent.profile_manager_mut()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
