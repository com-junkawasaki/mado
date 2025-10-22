//! ビデオ処理

use crate::{ProtocolMessage, MessageBuilder, MessageType, ProtocolError};
use soft_kvm_core::{Resolution, VideoQuality, KvmResult};
use std::sync::Arc;

/// ビデオマネージャー
pub struct VideoManager {
    message_builder: Arc<std::sync::Mutex<MessageBuilder>>,
}

impl VideoManager {
    pub fn new() -> Self {
        Self {
            message_builder: Arc::new(std::sync::Mutex::new(MessageBuilder::new())),
        }
    }

    /// ビデオフレームを送信
    pub async fn send_video_frame(&self, _data: &[u8], _width: u32, _height: u32) -> KvmResult<()> {
        // TODO: ビデオフレームのエンコードと送信を実装
        Ok(())
    }

    /// ビデオ設定を送信
    pub async fn send_video_config(&self, resolution: Resolution, quality: VideoQuality) -> KvmResult<()> {
        let config_msg = crate::VideoConfigMessage {
            resolution: Some(resolution.into()),
            quality: Some(quality.into()),
            codec: "h264".to_string(),
        };

        let payload = config_msg.encode_to_vec();
        let message = {
            let mut builder = self.message_builder.lock().unwrap();
            ProtocolMessage::new(MessageType::VideoConfig, payload)
                .with_sequence(builder.next_sequence())
        };

        // TODO: メッセージ送信を実装
        Ok(())
    }
}

impl Default for VideoManager {
    fn default() -> Self {
        Self::new()
    }
}
