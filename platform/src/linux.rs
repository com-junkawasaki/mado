//! Linux 固有の実装

pub mod video {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: Linux PipeWire/Wayland実装
        Ok(())
    }

    pub async fn start_display() -> KvmResult<()> {
        // TODO: Linux ビデオ表示実装
        Ok(())
    }
}

pub mod input {
    use soft_kvm_core::KvmResult;

    pub async fn start_capture() -> KvmResult<()> {
        // TODO: Linux libinput/evdev実装
        Ok(())
    }

    pub async fn start_processing() -> KvmResult<()> {
        // TODO: Linux 入力処理実装
        Ok(())
    }
}
