//! AI 模块 - LLM 交互（LiteLLM 统一接口）
//!
//! 核心职责：
//! - `AIController`: 实现 Handler trait，接收 AITask，调用 LLM，返回 AIResult
//! - `LiteLLMClient`: 统一接口调用多厂商 LLM

mod ai_controller;
mod llm_client;

pub use ai_controller::{AIController, AIResponse};
pub use llm_client::{LiteLLMClient, LLMClient, LLMResponse, StreamChunk, TokenUsage};
