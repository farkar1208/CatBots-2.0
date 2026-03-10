//! 会话代理 - 协调各模块处理用户输入

use crate::SessionState;
use catbots_ai::AIController;
use catbots_history::{ConversationTree, Handler, NodeProcessor, NodeType, ResultData};
use catbots_profile::{Profile, ProfileManager};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 会话代理
/// 
/// 核心职责：
/// - 协调 NodeProcessor 和 ProfileManager
/// - 处理用户输入
/// - 管理会话状态
/// - 管理配置切换
/// - 自动持久化历史记录
pub struct SessionAgent {
    /// 会话状态
    state: SessionState,
    /// Profile 管理器
    profile_manager: ProfileManager,
    /// 对话树（共享引用）
    tree: Arc<Mutex<ConversationTree>>,
    /// 节点处理器
    processor: NodeProcessor,
    /// AI 控制器（用于 Handler 注册）
    ai_controller: Arc<AIController>,
    /// 历史文件路径
    history_path: Option<PathBuf>,
}

impl SessionAgent {
    /// 创建新的会话代理
    pub fn new(profile_manager: ProfileManager) -> Self {
        let tree = Arc::new(Mutex::new(ConversationTree::new()));
        let ai_controller = Arc::new(AIController::new());
        
        // 获取默认 Profile 配置
        let (model, api_base, temperature, max_tokens) = profile_manager
            .get("default")
            .map(|p| {
                (
                    p.model.clone(),
                    p.api_base.clone(),
                    p.parameters.temperature,
                    p.parameters.max_tokens,
                )
            })
            .unwrap_or_else(|| {
                ("openai/gpt-4o".to_string(), None, Some(0.7), Some(4096))
            });

        let processor = NodeProcessor::new(tree.clone())
            .with_default_model(model)
            .with_default_api_base(api_base.unwrap_or_default())
            .with_default_temperature(temperature.unwrap_or(0.7))
            .with_default_max_tokens(max_tokens.unwrap_or(4096));

        // 注册 AI 处理器
        let mut processor = processor;
        processor.register_handler(NodeType::User, ai_controller.clone());

        Self {
            state: SessionState::new(),
            profile_manager,
            tree,
            processor,
            ai_controller,
            history_path: None,
        }
    }

    /// 使用现有对话树创建会话代理
    pub fn with_tree(profile_manager: ProfileManager, tree: Arc<Mutex<ConversationTree>>) -> Self {
        let ai_controller = Arc::new(AIController::new());
        
        let (model, api_base, temperature, max_tokens) = profile_manager
            .get("default")
            .map(|p| {
                (
                    p.model.clone(),
                    p.api_base.clone(),
                    p.parameters.temperature,
                    p.parameters.max_tokens,
                )
            })
            .unwrap_or_else(|| {
                ("openai/gpt-4o".to_string(), None, Some(0.7), Some(4096))
            });

        let processor = NodeProcessor::new(tree.clone())
            .with_default_model(model)
            .with_default_api_base(api_base.unwrap_or_default())
            .with_default_temperature(temperature.unwrap_or(0.7))
            .with_default_max_tokens(max_tokens.unwrap_or(4096));

        let mut processor = processor;
        processor.register_handler(NodeType::User, ai_controller.clone());
        
        Self {
            state: SessionState::new(),
            profile_manager,
            tree,
            processor,
            ai_controller,
            history_path: None,
        }
    }

    /// 设置历史文件路径并加载历史
    pub fn with_history_file(mut self, path: PathBuf) -> Result<Self, anyhow::Error> {
        // 加载历史
        let tree = ConversationTree::load_from_file(&path)?;
        let tree_arc = Arc::new(Mutex::new(tree));
        
        // 重新创建 processor
        let ai_controller = Arc::new(AIController::new());
        let mut processor = NodeProcessor::new(tree_arc.clone());
        processor.register_handler(NodeType::User, ai_controller.clone());
        
        self.tree = tree_arc;
        self.processor = processor;
        self.ai_controller = ai_controller;
        self.history_path = Some(path);
        
        Ok(self)
    }

    /// 创建使用默认历史文件路径的会话代理
    pub fn with_default_history(profile_manager: ProfileManager) -> Result<Self, anyhow::Error> {
        let history_path = Self::default_history_path()?;
        
        // 加载历史
        let tree = ConversationTree::load_from_file(&history_path)?;
        let tree_arc = Arc::new(Mutex::new(tree));
        
        let ai_controller = Arc::new(AIController::new());
        
        let (model, api_base, temperature, max_tokens) = profile_manager
            .get("default")
            .map(|p| {
                (
                    p.model.clone(),
                    p.api_base.clone(),
                    p.parameters.temperature,
                    p.parameters.max_tokens,
                )
            })
            .unwrap_or_else(|| {
                ("openai/gpt-4o".to_string(), None, Some(0.7), Some(4096))
            });

        let processor = NodeProcessor::new(tree_arc.clone())
            .with_default_model(model)
            .with_default_api_base(api_base.unwrap_or_default())
            .with_default_temperature(temperature.unwrap_or(0.7))
            .with_default_max_tokens(max_tokens.unwrap_or(4096));

        let mut processor = processor;
        processor.register_handler(NodeType::User, ai_controller.clone());
        
        Ok(Self {
            state: SessionState::new(),
            profile_manager,
            tree: tree_arc,
            processor,
            ai_controller,
            history_path: Some(history_path),
        })
    }

    /// 获取默认历史文件路径
    pub fn default_history_path() -> Result<PathBuf, anyhow::Error> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("catbots");
        
        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("history.json"))
    }

    /// 处理用户输入
    /// 
    /// 返回 AI 响应内容
    pub async fn process_input(&mut self, input: &str) -> Result<String, anyhow::Error> {
        // 1. 从 state 获取 currentNode
        let current_node_id = self.state.current_node_id.clone();
        
        // 2. 从 profile_manager 获取当前 Profile 配置
        let profile = self.get_current_profile()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No profile selected"))?;

        // 3. 更新 processor 配置
        self.processor.update_config(
            profile.model.clone(),
            profile.api_base.clone(),
            profile.parameters.temperature,
            profile.parameters.max_tokens,
        );

        // 4. 调用 tree.addUserNode(currentNode, input)
        let user_node_id = {
            let mut tree = self.tree.lock().await;
            tree.add_user_node(&current_node_id, input.to_string())
        };

        tracing::debug!(
            user_node_id = %user_node_id,
            profile = %profile.id,
            "用户节点已添加"
        );

        // 5. 调用 processor.requestProcess(userNodeId)
        let result = self.processor.request_process(&user_node_id).await?;

        // 6. 从结果中提取 AI 响应
        let (content, ai_node_id) = match result {
            ResultData::AI(ai_result) => {
                (ai_result.content, ai_result.node_id)
            }
            _ => return Err(anyhow::anyhow!("Unexpected result type")),
        };

        // 7. 更新 state.setCurrentNode(aiNodeId)
        self.state.set_current_node(ai_node_id);

        tracing::info!(
            ai_node_id = %self.state.current_node_id,
            "AI 响应已完成"
        );

        // 8. 自动保存历史
        self.save_history().await?;

        Ok(content)
    }

    /// 从指定节点创建分支
    pub async fn branch_from(&mut self, node_id: &str) -> Result<(), anyhow::Error> {
        {
            let tree = self.tree.lock().await;
            
            // 验证节点存在
            if tree.get_node(node_id).is_none() {
                return Err(anyhow::anyhow!("Node '{}' not found", node_id));
            }
        }
        
        // 设置当前节点为分支点
        self.state.set_current_node(node_id.to_string());
        
        // 保存历史
        self.save_history().await?;
        
        tracing::info!(node_id = %node_id, "已创建分支");
        Ok(())
    }

    /// 从指定节点创建分支（同步版本）
    pub fn branch_from_sync(&mut self, node_id: &str) -> Result<(), anyhow::Error> {
        {
            let tree = self.tree.blocking_lock();
            
            // 验证节点存在
            if tree.get_node(node_id).is_none() {
                return Err(anyhow::anyhow!("Node '{}' not found", node_id));
            }
        }
        
        // 设置当前节点为分支点
        self.state.set_current_node(node_id.to_string());
        
        tracing::info!(node_id = %node_id, "已创建分支");
        Ok(())
    }

    /// 切换 Profile
    pub fn switch_profile(&mut self, profile_id: &str) -> Result<(), anyhow::Error> {
        if self.profile_manager.get(profile_id).is_none() {
            return Err(anyhow::anyhow!("Profile '{}' not found", profile_id));
        }
        self.state.set_current_profile(profile_id.to_string());
        
        tracing::info!(profile_id = %profile_id, "已切换 Profile");
        Ok(())
    }

    /// 设置当前使用的模型
    pub fn set_model(&mut self, model: &str) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.model = model.to_string();
            self.profile_manager.update(modified_profile)?;
            tracing::info!(model = %model, "已设置模型");
        }
        
        Ok(())
    }

    /// 设置当前 Profile 的名称
    pub fn set_profile_name(&mut self, name: &str) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.name = name.to_string();
            self.profile_manager.update(modified_profile)?;
            tracing::info!(name = %name, "已设置 Profile 名称");
        }
        
        Ok(())
    }

    /// 设置当前 Profile 的温度参数
    pub fn set_temperature(&mut self, temperature: f32) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.parameters.temperature = Some(temperature);
            self.profile_manager.update(modified_profile)?;
            tracing::info!(temperature = temperature, "已设置温度");
        }
        
        Ok(())
    }

    /// 设置当前 Profile 的最大 token 数
    pub fn set_max_tokens(&mut self, max_tokens: u32) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.parameters.max_tokens = Some(max_tokens);
            self.profile_manager.update(modified_profile)?;
            tracing::info!(max_tokens = max_tokens, "已设置最大 token");
        }
        
        Ok(())
    }

    /// 设置当前 Profile 的 Top-P 参数
    pub fn set_top_p(&mut self, top_p: f32) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.parameters.top_p = Some(top_p);
            self.profile_manager.update(modified_profile)?;
            tracing::info!(top_p = top_p, "已设置 Top-P");
        }
        
        Ok(())
    }

    /// 设置当前 Profile 的 API 基础地址
    pub fn set_api_base(&mut self, api_base: &str) -> Result<(), anyhow::Error> {
        let current_profile_id = self.state.current_profile_id.clone();
        
        if let Some(profile) = self.profile_manager.get(&current_profile_id).cloned() {
            let mut modified_profile = profile;
            modified_profile.api_base = if api_base.is_empty() {
                None
            } else {
                Some(api_base.to_string())
            };
            self.profile_manager.update(modified_profile)?;
            tracing::info!(api_base = %api_base, "已设置 API 地址");
        }
        
        Ok(())
    }

    /// 创建新的 Profile
    pub fn create_profile(&mut self, profile_id: &str, model: Option<&str>) -> Result<(), anyhow::Error> {
        let model = model.unwrap_or("openai/gpt-4o");
        let profile = Profile::new(profile_id, profile_id, model);
        self.profile_manager.add(profile)?;
        tracing::info!(profile_id = %profile_id, model = %model, "已创建 Profile");
        Ok(())
    }

    /// 删除 Profile
    pub fn delete_profile(&mut self, profile_id: &str) -> Result<(), anyhow::Error> {
        // 不允许删除当前正在使用的 Profile
        if self.state.current_profile_id == profile_id {
            return Err(anyhow::anyhow!("Cannot delete the current profile"));
        }
        
        self.profile_manager.remove(profile_id)?;
        tracing::info!(profile_id = %profile_id, "已删除 Profile");
        Ok(())
    }

    /// 保存历史到文件
    pub async fn save_history(&self) -> Result<(), anyhow::Error> {
        if let Some(ref path) = self.history_path {
            let tree = self.tree.lock().await;
            tree.save_to_file(path)?;
        }
        Ok(())
    }

    /// 清除历史记录
    pub async fn clear_history(&mut self) -> Result<(), anyhow::Error> {
        {
            let mut tree = self.tree.lock().await;
            tree.clear();
        }
        self.state.set_current_node("root".to_string());
        self.save_history().await?;
        tracing::info!("已清除对话历史");
        Ok(())
    }

    /// 清除历史记录（同步版本）
    pub fn clear_history_sync(&mut self) -> Result<(), anyhow::Error> {
        {
            let mut tree = self.tree.blocking_lock();
            tree.clear();
        }
        self.state.set_current_node("root".to_string());
        tracing::info!("已清除对话历史");
        Ok(())
    }

    /// 获取当前会话状态
    pub fn get_current_state(&self) -> &SessionState {
        &self.state
    }

    /// 获取当前 Profile
    pub fn get_current_profile(&self) -> Option<&Profile> {
        self.profile_manager.get(&self.state.current_profile_id)
    }

    /// 获取 Profile 管理器
    pub fn profile_manager(&self) -> &ProfileManager {
        &self.profile_manager
    }

    /// 获取 Profile 管理器（可变）
    pub fn profile_manager_mut(&mut self) -> &mut ProfileManager {
        &mut self.profile_manager
    }

    /// 获取对话树引用
    pub fn tree(&self) -> Arc<Mutex<ConversationTree>> {
        self.tree.clone()
    }

    /// 获取节点处理器引用
    pub fn processor(&self) -> &NodeProcessor {
        &self.processor
    }

    /// 获取 AI 控制器引用
    pub fn ai_controller(&self) -> &AIController {
        &self.ai_controller
    }

    /// 获取当前节点下的子节点列表
    pub async fn get_children(&self, node_id: &str) -> Result<Vec<String>, anyhow::Error> {
        let tree = self.tree.lock().await;
        Ok(tree.get_children(node_id).iter().map(|s| s.to_string()).collect())
    }

    /// 获取当前节点下的子节点列表（同步版本）
    pub fn get_children_sync(&self, node_id: &str) -> Result<Vec<String>, anyhow::Error> {
        let tree = self.tree.blocking_lock();
        Ok(tree.get_children(node_id).iter().map(|s| s.to_string()).collect())
    }

    /// 获取对话历史路径
    pub async fn get_conversation_path(&self) -> Result<Vec<String>, anyhow::Error> {
        let tree = self.tree.lock().await;
        Ok(tree.get_path(&self.state.current_node_id))
    }

    /// 获取对话历史路径（同步版本）
    pub fn get_conversation_path_sync(&self) -> Result<Vec<String>, anyhow::Error> {
        let tree = self.tree.blocking_lock();
        Ok(tree.get_path(&self.state.current_node_id))
    }

    /// 注册自定义 Handler
    pub fn register_handler(&mut self, node_type: NodeType, handler: Arc<dyn Handler>) {
        self.processor.register_handler(node_type, handler);
    }
}
