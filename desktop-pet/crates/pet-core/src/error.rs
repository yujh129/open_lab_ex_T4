//! 领域错误类型。

use thiserror::Error;

/// pet-core 统一错误类型。
#[derive(Debug, Error)]
pub enum PetError {
    /// AI 对话引擎错误（网络、协议、模型侧）。
    #[error("ai error: {0}")]
    Ai(String),
    /// 持久化 / 检索错误。
    #[error("memory error: {0}")]
    Memory(String),
    /// 语音播报错误。
    #[error("voice error: {0}")]
    Voice(String),
    /// 提醒调度错误。
    #[error("reminder error: {0}")]
    Reminder(String),
    /// 调用方传入非法参数。
    #[error("invalid input: {0}")]
    Invalid(String),
    /// 其他未归类错误。
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// 便捷别名。
pub type PetResult<T> = std::result::Result<T, PetError>;
