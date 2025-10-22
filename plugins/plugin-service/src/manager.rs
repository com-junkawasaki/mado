//! サービスマネージャー

use crate::ServiceConfig;
use soft_kvm_core::KvmResult;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};

/// サービス状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

/// サービスマネージャー
pub struct ServiceManager {
    config: Arc<RwLock<ServiceConfig>>,
    state: Arc<RwLock<ServiceState>>,
}

impl ServiceManager {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(ServiceState::Stopped)),
        }
    }

    /// サービスを開始
    pub async fn start(&self) -> KvmResult<()> {
        let mut state = self.state.write().await;
        if *state != ServiceState::Stopped {
            return Err(soft_kvm_core::KvmError::Config("Service is not stopped".to_string()));
        }

        *state = ServiceState::Starting;
        info!("Starting Soft KVM service...");

        // プラットフォーム固有のサービス開始処理
        match self.start_platform_service().await {
            Ok(()) => {
                *state = ServiceState::Running;
                info!("Soft KVM service started successfully");
                Ok(())
            }
            Err(e) => {
                *state = ServiceState::Error;
                error!("Failed to start service: {}", e);
                Err(e)
            }
        }
    }

    /// サービスを停止
    pub async fn stop(&self) -> KvmResult<()> {
        let mut state = self.state.write().await;
        if *state != ServiceState::Running {
            return Err(soft_kvm_core::KvmError::Config("Service is not running".to_string()));
        }

        *state = ServiceState::Stopping;
        info!("Stopping Soft KVM service...");

        // プラットフォーム固有のサービス停止処理
        match self.stop_platform_service().await {
            Ok(()) => {
                *state = ServiceState::Stopped;
                info!("Soft KVM service stopped successfully");
                Ok(())
            }
            Err(e) => {
                *state = ServiceState::Error;
                error!("Failed to stop service: {}", e);
                Err(e)
            }
        }
    }

    /// サービス状態を取得
    pub async fn get_state(&self) -> ServiceState {
        *self.state.read().await
    }

    /// 設定を取得
    pub async fn get_config(&self) -> ServiceConfig {
        self.config.read().await.clone()
    }

    /// 設定を更新
    pub async fn update_config(&self, config: ServiceConfig) -> KvmResult<()> {
        let state = self.get_state().await;
        if state == ServiceState::Running {
            warn!("Updating configuration while service is running");
        }

        *self.config.write().await = config;
        Ok(())
    }

    /// プラットフォーム固有のサービス開始処理
    #[cfg(target_os = "linux")]
    async fn start_platform_service(&self) -> KvmResult<()> {
        crate::systemd::start_service().await
    }

    #[cfg(target_os = "macos")]
    async fn start_platform_service(&self) -> KvmResult<()> {
        crate::launchd::start_service().await
    }

    #[cfg(target_os = "windows")]
    async fn start_platform_service(&self) -> KvmResult<()> {
        crate::windows_svc::start_service().await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    async fn start_platform_service(&self) -> KvmResult<()> {
        Err(soft_kvm_core::KvmError::Platform("Unsupported platform".to_string()))
    }

    /// プラットフォーム固有のサービス停止処理
    #[cfg(target_os = "linux")]
    async fn stop_platform_service(&self) -> KvmResult<()> {
        crate::systemd::stop_service().await
    }

    #[cfg(target_os = "macos")]
    async fn stop_platform_service(&self) -> KvmResult<()> {
        crate::launchd::stop_service().await
    }

    #[cfg(target_os = "windows")]
    async fn stop_platform_service(&self) -> KvmResult<()> {
        crate::windows_svc::stop_service().await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    async fn stop_platform_service(&self) -> KvmResult<()> {
        Err(soft_kvm_core::KvmError::Platform("Unsupported platform".to_string()))
    }
}

/// サービスマネージャーのビルダー
pub struct ServiceManagerBuilder {
    config: ServiceConfig,
}

impl ServiceManagerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServiceConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ServiceConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.config.service.name = name.to_string();
        self
    }

    pub fn with_control_port(mut self, port: u16) -> Self {
        self.config.network.control_port = port;
        self
    }

    pub fn with_video_port(mut self, port: u16) -> Self {
        self.config.network.video_port = port;
        self
    }

    pub fn build(self) -> ServiceManager {
        ServiceManager::new(self.config)
    }
}

impl Default for ServiceManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
