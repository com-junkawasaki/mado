//! macOS 固有の実装

pub mod video {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: macOS Screen Capture Kit実装
        Ok(())
    }

    pub async fn start_display() -> KvmResult<()> {
        // TODO: macOS ビデオ表示実装
        Ok(())
    }
}

pub mod input {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: macOS 入力キャプチャ実装
        Ok(())
    }

    pub async fn start_processing() -> KvmResult<()> {
        // TODO: macOS 入力処理実装
        Ok(())
    }
}
