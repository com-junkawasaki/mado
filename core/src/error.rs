//! エラーハンドリング定義

use std::fmt;
use thiserror::Error;

/// Soft KVM全体のエラータイプ
#[derive(Debug, Error)]
pub enum KvmError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Video processing error: {0}")]
    Video(String),

    #[error("Input processing error: {0}")]
    Input(String),
}

/// 結果型エイリアス
pub type KvmResult<T> = Result<T, KvmError>;
