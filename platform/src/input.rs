//! 入力処理

use soft_kvm_core::{InputEvent, KvmResult};

/// 入力マネージャー
pub struct InputManager;

impl InputManager {
    pub fn new() -> Self {
        Self
    }

    /// 入力キャプチャを開始
    pub async fn start_capture(&self) -> KvmResult<()> {
        // TODO: プラットフォーム固有の入力キャプチャ実装
        Ok(())
    }

    /// 入力キャプチャを停止
    pub async fn stop_capture(&self) -> KvmResult<()> {
        // TODO: プラットフォーム固有の入力キャプチャ停止実装
        Ok(())
    }

    /// 入力イベントを送信
    pub async fn send_input(&self, _event: InputEvent) -> KvmResult<()> {
        // TODO: プラットフォーム固有の入力送信実装
        Ok(())
    }

    /// 次の入力イベントを取得
    pub async fn receive_input(&self) -> KvmResult<Option<InputEvent>> {
        // TODO: プラットフォーム固有の入力受信実装
        Ok(None)
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}
