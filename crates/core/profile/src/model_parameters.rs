//! 模型参数配置

use serde::{Deserialize, Serialize};

/// 模型参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelParameters {
    /// 温度参数
    pub temperature: Option<f32>,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
    /// Top-P 采样
    pub top_p: Option<f32>,
    /// 频率惩罚
    pub frequency_penalty: Option<f32>,
    /// 存在惩罚
    pub presence_penalty: Option<f32>,
    /// 停止序列
    pub stop_sequences: Option<Vec<String>>,
}
