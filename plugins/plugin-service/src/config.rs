//! サービス設定

use serde::{Deserialize, Serialize};
use soft_kvm_core::{NetworkAddress, VideoResolution, VideoQuality};
use std::path::PathBuf;

/// サービス設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service: ServiceSettings,
    pub network: NetworkSettings,
    pub video: VideoSettings,
    pub security: SecuritySettings,
    pub logging: LoggingSettings,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            service: ServiceSettings::default(),
            network: NetworkSettings::default(),
            video: VideoSettings::default(),
            security: SecuritySettings::default(),
            logging: LoggingSettings::default(),
        }
    }
}

/// サービス基本設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSettings {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub auto_start: bool,
    pub restart_on_failure: bool,
    pub restart_delay_seconds: u64,
}

impl Default for ServiceSettings {
    fn default() -> Self {
        Self {
            name: "soft-kvm".to_string(),
            display_name: "Soft KVM Service".to_string(),
            description: "LAN専用・低遅延KVM共有サービス".to_string(),
            auto_start: true,
            restart_on_failure: true,
            restart_delay_seconds: 5,
        }
    }
}

/// ネットワーク設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub bind_address: NetworkAddress,
    pub discovery_port: u16,
    pub control_port: u16,
    pub video_port: u16,
    pub max_connections: usize,
    pub connection_timeout_seconds: u64,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            bind_address: NetworkAddress::localhost(0), // OSが自動割り当て
            discovery_port: 5353, // mDNS標準ポート
            control_port: 8080,
            video_port: 8081,
            max_connections: 10,
            connection_timeout_seconds: 30,
        }
    }
}

/// ビデオ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub resolution: VideoResolution,
    pub quality: VideoQuality,
    pub capture_cursor: bool,
    pub capture_rate: u32, // FPS
    pub encoder_threads: usize,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            resolution: VideoResolution::fhd(),
            quality: VideoQuality::balanced(),
            capture_cursor: true,
            capture_rate: 30,
            encoder_threads: 4,
        }
    }
}

/// セキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub tls_enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub allowed_ips: Vec<String>, // CIDR表記
    pub session_timeout_minutes: u64,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            tls_enabled: true,
            cert_path: None,
            key_path: None,
            allowed_ips: vec!["192.168.0.0/16".to_string(), "10.0.0.0/8".to_string()],
            session_timeout_minutes: 60,
        }
    }
}

/// ログ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    pub level: String,
    pub file_path: Option<PathBuf>,
    pub max_file_size_mb: u64,
    pub max_files: usize,
    pub console_output: bool,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: Some(PathBuf::from("/var/log/soft-kvm.log")),
            max_file_size_mb: 100,
            max_files: 5,
            console_output: true,
        }
    }
}

/// 設定ローダー
pub struct ConfigLoader;

impl ConfigLoader {
    /// 設定ファイルを読み込み
    pub fn load_from_file(path: &PathBuf) -> soft_kvm_core::KvmResult<ServiceConfig> {
        let contents = std::fs::read_to_string(path)?;
        let config: ServiceConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// デフォルト設定で初期化
    pub fn create_default() -> ServiceConfig {
        ServiceConfig::default()
    }

    /// 設定をファイルに保存
    pub fn save_to_file(config: &ServiceConfig, path: &PathBuf) -> soft_kvm_core::KvmResult<()> {
        let contents = serde_json::to_string_pretty(config)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// 環境変数から設定を上書き
    pub fn override_from_env(mut config: ServiceConfig) -> ServiceConfig {
        if let Ok(port) = std::env::var("SOFT_KVM_CONTROL_PORT") {
            if let Ok(port_num) = port.parse() {
                config.network.control_port = port_num;
            }
        }

        if let Ok(port) = std::env::var("SOFT_KVM_VIDEO_PORT") {
            if let Ok(port_num) = port.parse() {
                config.network.video_port = port_num;
            }
        }

        if let Ok(log_level) = std::env::var("SOFT_KVM_LOG_LEVEL") {
            config.logging.level = log_level;
        }

        config
    }
}
