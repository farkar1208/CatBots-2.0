//! 命令解析器
//!
//! 采用对象前置设计，支持别名和引号参数

/// Profile 设置字段
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileSetField {
    Model,
    Name,
    Temperature,
    MaxTokens,
    TopP,
    ApiBase,
}

/// 命令类型
#[derive(Debug, Clone)]
pub enum Command {
    // === 消息发送 ===
    /// 发送消息
    Send { content: String },

    // === Profile 命令 ===
    /// 显示当前 Profile
    ProfileShow,
    /// 列出所有 Profile
    ProfileList,
    /// 切换 Profile
    ProfileSwitch { profile_id: String },
    /// 设置 Profile 属性
    ProfileSet { field: ProfileSetField, value: String },
    /// 创建 Profile
    ProfileCreate { profile_id: String, model: Option<String> },
    /// 删除 Profile
    ProfileDelete { profile_id: String },

    // === 模型命令 ===
    /// 显示当前模型
    ModelShow,
    /// 设置模型
    ModelSet { model: String },

    // === 对话管理 ===
    /// 显示对话历史
    HistoryShow,
    /// 显示对话树结构
    HistoryTree,
    /// 清除对话历史
    HistoryClear,
    /// 从节点创建分支
    Branch { node_id: String },
    /// 跳转到节点
    Goto { node_id: String },

    // === 状态 ===
    /// 显示完整状态摘要
    Status,

    // === 系统 ===
    /// 显示帮助
    Help,
    /// 清屏
    Clear,
    /// 退出
    Exit,

    /// 未知命令
    Unknown { input: String },
}

/// 命令解析器
pub struct CommandParser;

impl CommandParser {
    /// 创建新的命令解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析用户输入
    pub fn parse(&self, input: &str) -> Command {
        let trimmed = input.trim();
        
        // 命令以 / 开头
        if trimmed.starts_with('/') {
            self.parse_command(trimmed)
        } else if trimmed.is_empty() {
            Command::Unknown { input: String::new() }
        } else {
            // 普通消息
            Command::Send {
                content: trimmed.to_string(),
            }
        }
    }

    /// 解析命令
    fn parse_command(&self, input: &str) -> Command {
        // 使用智能分割，支持引号
        let parts = self.smart_split(input);
        
        if parts.is_empty() {
            return Command::Unknown {
                input: input.to_string(),
            };
        }

        match parts[0].as_str() {
            // === Profile 命令 ===
            "/profile" | "/p" => self.parse_profile_command(&parts),

            // === 模型命令 ===
            "/model" | "/m" => self.parse_model_command(&parts),

            // === 历史命令 ===
            "/history" | "/h" => self.parse_history_command(&parts),

            // === 分支命令 ===
            "/branch" => {
                if parts.len() < 2 {
                    Command::Unknown {
                        input: "用法: /branch <节点ID>".to_string(),
                    }
                } else {
                    Command::Branch { node_id: parts[1].clone() }
                }
            }

            // === 跳转命令 ===
            "/goto" => {
                if parts.len() < 2 {
                    Command::Unknown {
                        input: "用法: /goto <节点ID>".to_string(),
                    }
                } else {
                    Command::Goto { node_id: parts[1].clone() }
                }
            }

            // === 状态命令 ===
            "/status" => Command::Status,

            // === 系统命令 ===
            "/help" | "/?" => Command::Help,
            "/clear" => Command::Clear,
            "/exit" | "/quit" | "/q" => Command::Exit,

            // === 未知命令 ===
            _ => Command::Unknown {
                input: format!("未知命令: {}", parts[0]),
            },
        }
    }

    /// 解析 Profile 命令
    fn parse_profile_command(&self, parts: &[String]) -> Command {
        match parts.get(1).map(|s| s.as_str()) {
            // /profile, /p - 显示当前 profile
            None => Command::ProfileShow,

            // /profile list, /profile ls - 列出所有
            Some("list") | Some("ls") => Command::ProfileList,

            // /profile current, /profile show - 显示当前详情
            Some("current") | Some("show") => Command::ProfileShow,

            // /profile switch <id>, /profile use <id> - 切换
            Some("switch") | Some("use") => {
                if parts.len() < 3 {
                    Command::Unknown {
                        input: "用法: /profile switch <ProfileID>".to_string(),
                    }
                } else {
                    Command::ProfileSwitch { profile_id: parts[2].clone() }
                }
            }

            // /profile set <field> <value> - 设置属性
            Some("set") => {
                if parts.len() < 4 {
                    return Command::Unknown {
                        input: "用法: /profile set <字段> <值>\n字段: model, name, temperature, max_tokens, top_p, api_base".to_string(),
                    };
                }
                
                let field = match parts[2].as_str() {
                    "model" | "m" => ProfileSetField::Model,
                    "name" | "n" => ProfileSetField::Name,
                    "temperature" | "temp" | "t" => ProfileSetField::Temperature,
                    "max_tokens" | "tokens" | "max" => ProfileSetField::MaxTokens,
                    "top_p" | "top" => ProfileSetField::TopP,
                    "api_base" | "api" | "base" => ProfileSetField::ApiBase,
                    _ => {
                        return Command::Unknown {
                            input: format!("未知字段: {}。可用: model, name, temperature, max_tokens, top_p, api_base", parts[2]),
                        };
                    }
                };
                
                // 值取剩余部分
                let value = parts[3..].join(" ");
                Command::ProfileSet { field, value }
            }

            // /profile create <id> [model] - 创建
            Some("create") | Some("new") => {
                if parts.len() < 3 {
                    Command::Unknown {
                        input: "用法: /profile create <ProfileID> [模型]".to_string(),
                    }
                } else {
                    let profile_id = parts[2].clone();
                    let model = parts.get(3).cloned();
                    Command::ProfileCreate { profile_id, model }
                }
            }

            // /profile delete <id>, /profile del <id>, /profile rm <id> - 删除
            Some("delete") | Some("del") | Some("rm") => {
                if parts.len() < 3 {
                    Command::Unknown {
                        input: "用法: /profile delete <ProfileID>".to_string(),
                    }
                } else {
                    Command::ProfileDelete { profile_id: parts[2].clone() }
                }
            }

            // /profile <id> - 直接切换（作为快捷方式）
            Some(id) => Command::ProfileSwitch { profile_id: id.to_string() },
        }
    }

    /// 解析模型命令
    fn parse_model_command(&self, parts: &[String]) -> Command {
        match parts.len() {
            // /model, /m - 显示当前模型
            1 => Command::ModelShow,
            // /model <name> - 设置模型
            _ => {
                let model = parts[1..].join(" ");
                Command::ModelSet { model }
            }
        }
    }

    /// 解析历史命令
    fn parse_history_command(&self, parts: &[String]) -> Command {
        match parts.get(1).map(|s| s.as_str()) {
            // /history tree - 显示树结构
            Some("tree") => Command::HistoryTree,
            // /history clear - 清除历史
            Some("clear") => Command::HistoryClear,
            // /history, /h - 显示历史
            _ => Command::HistoryShow,
        }
    }

    /// 智能分割，支持引号包裹的参数
    fn smart_split(&self, input: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = c;
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                    quote_char = ' ';
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current);
        }

        parts
    }

    /// 获取帮助文本
    pub fn help_text() -> &'static str {
        r#"
┌─────────────────────────────────────────────────────────────────────────┐
│ 消息                                                                    │
├─────────────────────────────────────────────────────────────────────────┤
│  <内容>                          发送消息给 AI                           │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│ Profile 配置 (别名: /p)                                                 │
├─────────────────────────────────────────────────────────────────────────┤
│  /profile                        显示当前 Profile                       │
│  /profile list                   列出所有 Profile                       │
│  /profile <id>                   切换到指定 Profile                      │
│  /profile switch <id>            切换 Profile（显式）                   │
│  /profile set <字段> <值>        设置属性                               │
│  /profile create <id> [模型]     创建新 Profile                         │
│  /profile delete <id>            删除 Profile                           │
│                                                                         │
│  可设置字段: model, name, temperature, max_tokens, top_p, api_base     │
│  示例: /profile set temperature 0.8                                     │
│        /profile set name "我的配置"                                     │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│ 模型 (别名: /m)                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│  /model                          显示当前模型                           │
│  /model <名称>                   设置模型                               │
│  示例: /model openai/gpt-4o-mini                                        │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│ 对话管理                                                                │
├─────────────────────────────────────────────────────────────────────────┤
│  /history, /h                    显示对话历史                           │
│  /history tree                   显示对话树结构                         │
│  /history clear                  清除对话历史                           │
│  /branch <节点ID>                从节点创建分支                         │
│  /goto <节点ID>                  跳转到节点                             │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│ 状态与系统                                                              │
├─────────────────────────────────────────────────────────────────────────┤
│  /status                         显示完整状态摘要                       │
│  /help, /?                       显示帮助                               │
│  /clear                          清屏                                   │
│  /exit, /quit, /q                退出程序                               │
└─────────────────────────────────────────────────────────────────────────┘
"#
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_split() {
        let parser = CommandParser::new();
        
        // 普通分割
        let parts = parser.smart_split("/profile set name test");
        assert_eq!(parts, vec!["/profile", "set", "name", "test"]);
        
        // 引号包裹
        let parts = parser.smart_split("/profile set name \"my name\"");
        assert_eq!(parts, vec!["/profile", "set", "name", "my name"]);
        
        // 单引号
        let parts = parser.smart_split("/profile set name 'my name'");
        assert_eq!(parts, vec!["/profile", "set", "name", "my name"]);
    }

    #[test]
    fn test_model_command() {
        let parser = CommandParser::new();
        
        // 显示模型
        let cmd = parser.parse("/model");
        assert!(matches!(cmd, Command::ModelShow));
        
        // 设置模型
        let cmd = parser.parse("/model gpt-4o");
        assert!(matches!(cmd, Command::ModelSet { model } if model == "gpt-4o"));
        
        // 别名
        let cmd = parser.parse("/m");
        assert!(matches!(cmd, Command::ModelShow));
    }

    #[test]
    fn test_profile_command() {
        let parser = CommandParser::new();
        
        // 显示
        let cmd = parser.parse("/profile");
        assert!(matches!(cmd, Command::ProfileShow));
        
        // 列表
        let cmd = parser.parse("/profile list");
        assert!(matches!(cmd, Command::ProfileList));
        
        // 切换
        let cmd = parser.parse("/profile claude");
        assert!(matches!(cmd, Command::ProfileSwitch { profile_id } if profile_id == "claude"));
        
        // 设置
        let cmd = parser.parse("/profile set temperature 0.8");
        assert!(matches!(cmd, Command::ProfileSet { field: ProfileSetField::Temperature, value } if value == "0.8"));
        
        // 设置带引号
        let cmd = parser.parse("/profile set name \"my profile\"");
        assert!(matches!(cmd, Command::ProfileSet { field: ProfileSetField::Name, value } if value == "my profile"));
    }

    #[test]
    fn test_send_message() {
        let parser = CommandParser::new();
        
        let cmd = parser.parse("Hello, how are you?");
        assert!(matches!(cmd, Command::Send { content } if content == "Hello, how are you?"));
    }
}