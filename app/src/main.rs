//! CatBots 2.0 - AI 对话机器人
//!
//! 基于 Rust 实现的 AI 对话系统，支持：
//! - 对话树管理与分支
//! - 多种 LLM 后端
//! - 终端交互界面

use anyhow::Result;
use catbots_agent::SessionAgent;
use catbots_config::ConfigManager;
use catbots_profile::{FileStorage, ModelParameters, Profile, ProfileManager};
use catbots_terminal::TerminalUI;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("CatBots 2.0 启动中...");

    // 加载配置
    let _config = ConfigManager::new();
    tracing::info!("配置加载完成");

    // 创建 Profile 管理器（使用文件存储）
    let profile_manager = init_profiles()?;
    tracing::info!("Profile 初始化完成");

    // 创建会话代理（自动加载历史）
    let agent = SessionAgent::with_default_history(profile_manager)?;
    tracing::info!("会话代理创建完成（历史已加载）");

    // 创建终端 UI 并设置代理
    let mut ui = TerminalUI::new().with_agent(agent);
    
    tracing::info!("启动终端 UI...");
    
    // 启动终端 UI
    ui.run().await?;

    tracing::info!("CatBots 2.0 已退出");
    Ok(())
}

/// 初始化 Profile 管理器
/// 
/// 首次运行时会创建默认 Profile，之后从文件加载
fn init_profiles() -> Result<ProfileManager> {
    // 尝试从文件加载
    match ProfileManager::with_file_storage() {
        Ok(manager) => {
            // 如果已有 Profile，直接返回
            if !manager.list().is_empty() {
                tracing::info!(
                    count = manager.list().len(),
                    "从文件加载 Profile"
                );
                return Ok(manager);
            }
            
            // 文件存在但为空，初始化默认 Profile
            tracing::info!("Profile 文件为空，初始化默认配置");
            let mut manager = manager;
            add_default_profiles(&mut manager)?;
            return Ok(manager);
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "无法加载 Profile 文件，将创建新文件"
            );
        }
    }
    
    // 创建新的管理器并初始化默认 Profile
    let storage = FileStorage::default_path()?;
    let mut manager = ProfileManager::new(Box::new(storage))?;
    add_default_profiles(&mut manager)?;
    Ok(manager)
}

/// 添加默认 Profile
fn add_default_profiles(manager: &mut ProfileManager) -> Result<()> {
    // 默认 Profile - OpenAI GPT-4o
    let default_profile = Profile::new("default", "默认 (GPT-4o)", "openai/gpt-4o")
        .as_default()
        .with_parameters(ModelParameters {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            ..Default::default()
        });
    manager.add(default_profile)?;
    
    // GPT-4o Mini - 更快更便宜
    let mini_profile = Profile::new("mini", "GPT-4o Mini", "openai/gpt-4o-mini")
        .with_parameters(ModelParameters {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            ..Default::default()
        });
    manager.add(mini_profile)?;
    
    // Claude Sonnet 4
    let claude_profile = Profile::new("claude", "Claude Sonnet 4", "anthropic/claude-sonnet-4-20250514")
        .with_parameters(ModelParameters {
            temperature: Some(1.0),
            max_tokens: Some(4096),
            ..Default::default()
        });
    manager.add(claude_profile)?;
    
    // 本地 Ollama 模型
    let ollama_profile = Profile::new("local", "本地 Llama", "ollama/llama3.1:8b")
        .with_api_base("http://localhost:11434/v1")
        .with_parameters(ModelParameters {
            temperature: Some(0.8),
            max_tokens: Some(2048),
            ..Default::default()
        });
    manager.add(ollama_profile)?;
    
    Ok(())
}
