//! 终端 UI 实现

use crate::{Command, CommandParser, ProfileSetField};
use catbots_agent::SessionAgent;
use std::io::{self, Write};

/// 终端 UI
/// 
/// 核心职责：
/// - 显示对话界面
/// - 接收用户输入
/// - 解析命令
/// - 协调 SessionAgent
pub struct TerminalUI {
    /// 命令解析器
    command_parser: CommandParser,
    /// 是否运行中
    running: bool,
    /// 会话代理
    agent: Option<SessionAgent>,
}

impl TerminalUI {
    /// 创建新的终端 UI
    pub fn new() -> Self {
        Self {
            command_parser: CommandParser::new(),
            running: false,
            agent: None,
        }
    }

    /// 设置会话代理
    pub fn with_agent(mut self, agent: SessionAgent) -> Self {
        self.agent = Some(agent);
        self
    }

    /// 启动 UI 主循环
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        self.running = true;

        // 显示欢迎界面
        self.show_welcome();

        // 显示当前配置
        self.show_current_config();

        // 主循环
        while self.running {
            // 读取输入（捕获错误，不退出）
            let input = match self.read_input() {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("读取输入失败: {}", e);
                    continue;
                }
            };

            if input.is_empty() {
                continue;
            }

            // 解析命令
            let command = self.command_parser.parse(&input);

            // 执行命令（捕获错误，不退出）
            if let Err(e) = self.execute_command(command).await {
                eprintln!("错误: {}", e);
            }
        }

        Ok(())
    }

    /// 停止 UI
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// 显示欢迎界面
    fn show_welcome(&self) {
        println!();
        println!("╔═══════════════════════════════════════════╗");
        println!("║         CatBots 2.0 - AI Chat Bot         ║");
        println!("╚═══════════════════════════════════════════╝");
        println!();
        println!("输入 /help 查看可用命令");
    }

    /// 显示当前配置
    fn show_current_config(&self) {
        if let Some(agent) = &self.agent {
            if let Some(profile) = agent.get_current_profile() {
                println!();
                println!("当前配置:");
                println!("  Profile: {} ({})", profile.name, profile.id);
                println!("  Model:   {}", profile.model);
                println!();
            }
        }
    }

    /// 读取用户输入
    fn read_input(&self) -> Result<String, anyhow::Error> {
        // 显示当前模型提示
        if let Some(agent) = &self.agent {
            if let Some(profile) = agent.get_current_profile() {
                print!("[{}] > ", profile.model_name());
            } else {
                print!("> ");
            }
        } else {
            print!("> ");
        }
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    /// 执行命令
    async fn execute_command(&mut self, command: Command) -> Result<(), anyhow::Error> {
        match command {
            // === 消息发送 ===
            Command::Send { content } => {
                self.handle_send(&content).await?;
            }

            // === Profile 命令 ===
            Command::ProfileShow => {
                self.handle_profile_show()?;
            }
            Command::ProfileList => {
                self.handle_profile_list()?;
            }
            Command::ProfileSwitch { profile_id } => {
                self.handle_profile_switch(&profile_id)?;
            }
            Command::ProfileSet { field, value } => {
                self.handle_profile_set(field, &value)?;
            }
            Command::ProfileCreate { profile_id, model } => {
                self.handle_profile_create(&profile_id, model.as_deref())?;
            }
            Command::ProfileDelete { profile_id } => {
                self.handle_profile_delete(&profile_id)?;
            }

            // === 模型命令 ===
            Command::ModelShow => {
                self.handle_model_show()?;
            }
            Command::ModelSet { model } => {
                self.handle_model_set(&model)?;
            }

            // === 对话管理 ===
            Command::HistoryShow => {
                self.handle_history_show()?;
            }
            Command::HistoryTree => {
                self.handle_history_tree()?;
            }
            Command::HistoryClear => {
                self.handle_history_clear()?;
            }
            Command::Branch { node_id } => {
                self.handle_branch(&node_id)?;
            }
            Command::Goto { node_id } => {
                self.handle_goto(&node_id)?;
            }

            // === Tag 标签命令 ===
            Command::TagList { node_id } => {
                self.handle_tag_list(node_id.as_deref())?;
            }
            Command::TagAdd { node_id, tag_id } => {
                self.handle_tag_add(&node_id, &tag_id)?;
            }
            Command::TagRevoke { instance_id, reason } => {
                self.handle_tag_revoke(&instance_id, reason.as_deref())?;
            }
            Command::TagSchemaList => {
                self.handle_tag_schema_list()?;
            }
            Command::TagSchemaShow { tag_id } => {
                self.handle_tag_schema_show(&tag_id)?;
            }
            Command::TagValidate { node_id, node_type } => {
                self.handle_tag_validate(&node_id, node_type.as_deref())?;
            }

            // === 状态 ===
            Command::Status => {
                self.handle_status()?;
            }

            // === 系统 ===
            Command::Help => {
                self.show_help();
            }
            Command::Clear => {
                self.clear_screen();
            }
            Command::Exit => {
                self.running = false;
                println!("再见！");
            }

            // === 未知命令 ===
            Command::Unknown { input } => {
                if !input.is_empty() {
                    println!("{}", input);
                }
                println!("输入 /help 查看可用命令");
            }
        }
        Ok(())
    }

    // ============================================================
    // 消息发送
    // ============================================================

    async fn handle_send(&mut self, content: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            // 显示用户消息
            println!();
            println!("┌─ 你 ─────────────────────────────────────");
            println!("{}", content);
            println!("└──────────────────────────────────────────");
            println!();

            // 显示思考状态
            print!("🤔 思考中...");
            io::stdout().flush()?;

            // 处理输入
            let response = agent.process_input(content).await;

            // 清除思考状态并显示响应
            print!("\r");
            println!("┌─ AI ─────────────────────────────────────");
            match response {
                Ok(content) => {
                    println!("{}", content);
                }
                Err(e) => {
                    println!("❌ 错误: {}", e);
                }
            }
            println!("└──────────────────────────────────────────");
            println!();
        } else {
            println!("错误: 会话代理未初始化");
        }
        Ok(())
    }

    // ============================================================
    // Profile 命令
    // ============================================================

    fn handle_profile_show(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            println!();
            
            if let Some(profile) = agent.get_current_profile() {
                println!("当前 Profile:");
                println!("──────────────────────────────────────────");
                println!("  ID:     {}", profile.id);
                println!("  名称:   {}", profile.name);
                println!("  模型:   {}", profile.model);
                
                if let Some(ref api_base) = profile.api_base {
                    println!("  API:    {}", api_base);
                }
                
                println!();
                println!("模型参数:");
                if let Some(temp) = profile.parameters.temperature {
                    println!("  temperature: {}", temp);
                }
                if let Some(tokens) = profile.parameters.max_tokens {
                    println!("  max_tokens:   {}", tokens);
                }
                if let Some(top_p) = profile.parameters.top_p {
                    println!("  top_p:        {}", top_p);
                }
                
                println!("──────────────────────────────────────────");
            } else {
                println!("未设置当前 Profile");
            }
            println!();
        }
        Ok(())
    }

    fn handle_profile_list(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            let profiles = agent.profile_manager().list();
            
            println!();
            println!("Profile 列表:");
            println!("──────────────────────────────────────────");
            
            if profiles.is_empty() {
                println!("(暂无 Profile)");
            } else {
                let current_id = agent.get_current_state().current_profile();
                
                for profile in profiles {
                    let current_marker = if profile.id == current_id { " *" } else { "" };
                    let default_marker = if profile.is_default { " [默认]" } else { "" };
                    
                    println!("  {}{}{}", profile.id, current_marker, default_marker);
                    println!("    名称: {}", profile.name);
                    println!("    模型: {}", profile.model);
                    
                    if let Some(ref api_base) = profile.api_base {
                        println!("    API:  {}", api_base);
                    }
                    println!();
                }
            }
            
            println!("──────────────────────────────────────────");
        }
        Ok(())
    }

    fn handle_profile_switch(&mut self, profile_id: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.switch_profile(profile_id)?;
            
            if let Some(profile) = agent.get_current_profile() {
                println!("✓ 已切换到 Profile: {} ({})", profile.name, profile.id);
                println!("  Model: {}", profile.model);
            }
        }
        Ok(())
    }

    fn handle_profile_set(&mut self, field: ProfileSetField, value: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            match field {
                ProfileSetField::Model => {
                    agent.set_model(value)?;
                    println!("✓ 已设置模型: {}", value);
                }
                ProfileSetField::Name => {
                    agent.set_profile_name(value)?;
                    println!("✓ 已设置名称: {}", value);
                }
                ProfileSetField::Temperature => {
                    let temp: f32 = value.parse()
                        .map_err(|_| anyhow::anyhow!("无效的温度值，请输入 0-2 之间的数字"))?;
                    if temp < 0.0 || temp > 2.0 {
                        return Err(anyhow::anyhow!("温度值必须在 0-2 之间"));
                    }
                    agent.set_temperature(temp)?;
                    println!("✓ 已设置温度: {}", temp);
                }
                ProfileSetField::MaxTokens => {
                    let tokens: u32 = value.parse()
                        .map_err(|_| anyhow::anyhow!("无效的 token 数，请输入正整数"))?;
                    agent.set_max_tokens(tokens)?;
                    println!("✓ 已设置最大 token: {}", tokens);
                }
                ProfileSetField::TopP => {
                    let top_p: f32 = value.parse()
                        .map_err(|_| anyhow::anyhow!("无效的 Top-P 值，请输入 0-1 之间的数字"))?;
                    if top_p < 0.0 || top_p > 1.0 {
                        return Err(anyhow::anyhow!("Top-P 值必须在 0-1 之间"));
                    }
                    agent.set_top_p(top_p)?;
                    println!("✓ 已设置 Top-P: {}", top_p);
                }
                ProfileSetField::ApiBase => {
                    agent.set_api_base(value)?;
                    if value.is_empty() {
                        println!("✓ 已清除 API 地址");
                    } else {
                        println!("✓ 已设置 API 地址: {}", value);
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_profile_create(&mut self, profile_id: &str, model: Option<&str>) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.create_profile(profile_id, model)?;
            let model = model.unwrap_or("openai/gpt-4o");
            println!("✓ 已创建 Profile: {} (模型: {})", profile_id, model);
        }
        Ok(())
    }

    fn handle_profile_delete(&mut self, profile_id: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.delete_profile(profile_id)?;
            println!("✓ 已删除 Profile: {}", profile_id);
        }
        Ok(())
    }

    // ============================================================
    // 模型命令
    // ============================================================

    fn handle_model_show(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            if let Some(profile) = agent.get_current_profile() {
                println!("当前模型: {}", profile.model);
            }
        }
        Ok(())
    }

    fn handle_model_set(&mut self, model: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.set_model(model)?;
            println!("✓ 已设置模型: {}", model);
        }
        Ok(())
    }

    // ============================================================
    // 对话管理
    // ============================================================

    fn handle_history_show(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            let path = agent.get_conversation_path_sync()?;
            
            println!();
            println!("对话历史:");
            println!("──────────────────────────────────────────");
            
            if path.is_empty() || (path.len() == 1 && path[0] == "root") {
                println!("(暂无对话)");
            } else {
                for (i, node_id) in path.iter().enumerate() {
                    if node_id != "root" {
                        println!("  [{}] {}", i, node_id);
                    }
                }
            }
            
            println!("──────────────────────────────────────────");
            println!();
        }
        Ok(())
    }

    fn handle_history_tree(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            println!();
            println!("对话树结构:");
            println!("──────────────────────────────────────────");
            
            // 使用 agent 的方法获取树信息
            let path = agent.get_conversation_path_sync()?;
            
            if path.is_empty() || (path.len() == 1 && path[0] == "root") {
                println!("📁 root");
                println!("  (暂无对话)");
            } else {
                println!("📁 root");
                for (i, node_id) in path.iter().enumerate() {
                    if node_id != "root" {
                        // 简单显示节点ID，实际内容需要通过 tree 获取
                        let indent = "  ".repeat(i);
                        let icon = if i % 2 == 1 { "👤" } else { "🤖" };
                        println!("{}{} {}", indent, icon, node_id);
                    }
                }
            }
            
            println!("──────────────────────────────────────────");
            println!();
        }
        Ok(())
    }

    fn handle_branch(&mut self, node_id: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.branch_from_sync(node_id)?;
            println!("✓ 已从节点 {} 创建分支", node_id);
        }
        Ok(())
    }

    fn handle_goto(&mut self, node_id: &str) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.branch_from_sync(node_id)?;
            println!("✓ 已跳转到节点 {}", node_id);
        }
        Ok(())
    }

    fn handle_history_clear(&mut self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &mut self.agent {
            agent.clear_history_sync()?;
            println!("✓ 已清除对话历史");
        }
        Ok(())
    }

    // ============================================================
    // 状态
    // ============================================================

    fn handle_status(&self) -> Result<(), anyhow::Error> {
        if let Some(agent) = &self.agent {
            println!();
            println!("状态摘要:");
            println!("──────────────────────────────────────────");
            
            // Profile 信息
            if let Some(profile) = agent.get_current_profile() {
                println!("  Profile:  {} ({})", profile.name, profile.id);
                println!("  Model:    {}", profile.model);
            }
            
            // 节点信息
            let state = agent.get_current_state();
            println!("  节点:     {}", state.current_node_id);
            
            // 对话数
            let path = agent.get_conversation_path_sync()?;
            let msg_count = path.iter().filter(|id| *id != "root").count();
            println!("  对话数:   {}", msg_count);
            
            println!("──────────────────────────────────────────");
            println!();
        }
        Ok(())
    }

    // ============================================================
    // Tag 标签命令
    // ============================================================

    fn handle_tag_list(&self, node_id: Option<&str>) -> Result<(), anyhow::Error> {
        println!();
        println!("标签列表:");
        println!("──────────────────────────────────────────");
        
        let target_node = node_id.map(|s| s.to_string())
            .or_else(|| self.agent.as_ref().map(|a| a.get_current_state().current_node_id.clone()));
        
        if let Some(node) = target_node {
            println!("节点: {}", node);
            println!();
            println!("(Tag 系统尚未完全集成，此处为占位符)");
        } else {
            println!("错误: 无法确定目标节点");
        }
        
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    fn handle_tag_add(&self, node_id: &str, tag_id: &str) -> Result<(), anyhow::Error> {
        println!();
        println!("添加标签:");
        println!("──────────────────────────────────────────");
        println!("节点: {}", node_id);
        println!("标签: {}", tag_id);
        println!();
        println!("(Tag 系统尚未完全集成，此处为占位符)");
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    fn handle_tag_revoke(&self, instance_id: &str, reason: Option<&str>) -> Result<(), anyhow::Error> {
        println!();
        println!("撤销标签:");
        println!("──────────────────────────────────────────");
        println!("实例 ID: {}", instance_id);
        if let Some(r) = reason {
            println!("原因: {}", r);
        }
        println!();
        println!("(Tag 系统尚未完全集成，此处为占位符)");
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    fn handle_tag_schema_list(&self) -> Result<(), anyhow::Error> {
        println!();
        println!("标签 Schema 列表:");
        println!("──────────────────────────────────────────");
        println!("(Tag 系统尚未完全集成，此处为占位符)");
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    fn handle_tag_schema_show(&self, tag_id: &str) -> Result<(), anyhow::Error> {
        println!();
        println!("标签 Schema 详情:");
        println!("──────────────────────────────────────────");
        println!("标签 ID: {}", tag_id);
        println!();
        println!("(Tag 系统尚未完全集成，此处为占位符)");
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    fn handle_tag_validate(&self, node_id: &str, node_type: Option<&str>) -> Result<(), anyhow::Error> {
        println!();
        println!("验证节点创建:");
        println!("──────────────────────────────────────────");
        println!("节点 ID: {}", node_id);
        if let Some(t) = node_type {
            println!("节点类型: {}", t);
        }
        println!();
        println!("(Tag 系统尚未完全集成，此处为占位符)");
        println!("──────────────────────────────────────────");
        println!();
        Ok(())
    }

    // ============================================================
    // 系统命令
    // ============================================================

    fn show_help(&self) {
        println!("{}", CommandParser::help_text());
    }

    /// 清屏
    pub fn clear_screen(&self) {
        // 使用 ANSI 转义序列清屏
        print!("\x1B[2J\x1B[1;1H");
        // 如果 ANSI 不工作，打印多个空行
        for _ in 0..50 {
            println!();
        }
    }
}

impl Default for TerminalUI {
    fn default() -> Self {
        Self::new()
    }
}