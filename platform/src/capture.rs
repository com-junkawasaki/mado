//! 画面キャプチャ

use soft_kvm_core::KvmResult;

/// キャプチャマネージャー
pub struct CaptureManager;

impl CaptureManager {
    pub fn new() -> Self {
        Self
    }

    /// 画面キャプチャを開始
    pub async fn start_capture(&self) -> KvmResult<()> {
        // TODO: プラットフォーム固有のキャプチャ実装
        Ok(())
    }

    /// 画面キャプチャを停止
    pub async fn stop_capture(&self) -> KvmResult<()> {
        // TODO: プラットフォーム固有のキャプチャ停止実装
        Ok(())
    }

    /// フレームを取得
    pub async fn capture_frame(&self) -> KvmResult<Vec<u8>> {
        // TODO: プラットフォーム固有のフレーム取得実装
        Ok(vec![])
    }
}

impl Default for CaptureManager {
    fn default() -> Self {
        Self::new()
    }
}
