//! Terminal UI 模块 - 终端交互界面
//!
//! 核心职责：
//! - `TerminalUI`: 终端交互
//! - `CommandParser`: 命令解析

mod terminal_ui;
mod command_parser;

pub use command_parser::{Command, CommandParser, ProfileSetField};
pub use terminal_ui::TerminalUI;
