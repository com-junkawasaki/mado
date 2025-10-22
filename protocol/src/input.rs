//! 入力処理

use crate::{ProtocolMessage, MessageBuilder, MessageType, ProtocolError};
use soft_kvm_core::{InputEvent, KvmResult};
use std::sync::Arc;

/// 入力マネージャー
pub struct InputManager {
    message_builder: Arc<std::sync::Mutex<MessageBuilder>>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            message_builder: Arc::new(std::sync::Mutex::new(MessageBuilder::new())),
        }
    }

    /// 入力イベントを送信
    pub async fn send_input_event(&self, event: InputEvent) -> KvmResult<()> {
        let message = {
            let mut builder = self.message_builder.lock().unwrap();
            builder.build_input_event(event)
        };

        // TODO: メッセージ送信を実装
        Ok(())
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}
