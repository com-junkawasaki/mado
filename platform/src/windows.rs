//! Windows 固有の実装

pub mod video {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: Windows Desktop Duplication API実装
        Ok(())
    }

    pub async fn start_display() -> KvmResult<()> {
        // TODO: Windows ビデオ表示実装
        Ok(())
    }
}

pub mod input {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: Windows Raw Input API実装
        Ok(())
    }

    pub async fn start_processing() -> KvmResult<()> {
        // TODO: Windows 入力処理実装
        Ok(())
    }
}
