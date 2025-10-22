//! 制御チャネル

use crate::{ProtocolMessage, MessageBuilder, MessageType, ProtocolError};
use soft_kvm_core::KvmResult;
use std::sync::Arc;

/// 制御マネージャー
pub struct ControlManager {
    message_builder: Arc<std::sync::Mutex<MessageBuilder>>,
}

impl ControlManager {
    pub fn new() -> Self {
        Self {
            message_builder: Arc::new(std::sync::Mutex::new(MessageBuilder::new())),
        }
    }

    /// ハートビートを送信
    pub async fn send_heartbeat(&self) -> KvmResult<()> {
        let message = {
            let mut builder = self.message_builder.lock().unwrap();
            builder.build_heartbeat()
        };

        // TODO: メッセージ送信を実装
        Ok(())
    }

    /// エラーを送信
    pub async fn send_error(&self, code: u32, message: &str, fatal: bool) -> KvmResult<()> {
        let error_message = {
            let mut builder = self.message_builder.lock().unwrap();
            builder.build_error(code, message, fatal)
        };

        // TODO: メッセージ送信を実装
        Ok(())
    }
}

impl Default for ControlManager {
    fn default() -> Self {
        Self::new()
    }
}
