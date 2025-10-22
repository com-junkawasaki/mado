//! 権限管理と要求

use soft_kvm_core::KvmResult;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// 必要な権限種別
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    /// 画面キャプチャ権限
    ScreenCapture,
    /// 入力デバイスアクセス権限
    InputDevices,
    /// ネットワークアクセス権限
    NetworkAccess,
    /// ファイルシステムアクセス権限
    FileSystemAccess,
    /// システムサービス権限
    SystemService,
}

/// 権限要求結果
#[derive(Debug, Clone)]
pub enum PermissionStatus {
    Granted,
    Denied,
    Pending,
    NotApplicable,
}

/// 権限マネージャー
pub struct PermissionManager {
    granted_permissions: HashSet<Permission>,
    required_permissions: Vec<Permission>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            granted_permissions: HashSet::new(),
            required_permissions: vec![
                Permission::ScreenCapture,
                Permission::InputDevices,
                Permission::NetworkAccess,
            ],
        }
    }

    /// 必要な全権限を要求
    pub async fn request_all_permissions(&mut self) -> KvmResult<()> {
        info!("Requesting all required permissions...");

        for permission in &self.required_permissions.clone() {
            match self.request_permission(permission.clone()).await {
                Ok(status) => match status {
                    PermissionStatus::Granted => {
                        info!("Permission granted: {:?}", permission);
                        self.granted_permissions.insert(permission.clone());
                    }
                    PermissionStatus::Denied => {
                        error!("Permission denied: {:?}", permission);
                        return Err(soft_kvm_core::KvmError::Platform(
                            format!("Required permission denied: {:?}", permission)
                        ));
                    }
                    PermissionStatus::Pending => {
                        warn!("Permission pending: {:?}", permission);
                    }
                    PermissionStatus::NotApplicable => {
                        debug!("Permission not applicable: {:?}", permission);
                    }
                },
                Err(e) => {
                    error!("Failed to request permission {:?}: {}", permission, e);
                    return Err(e);
                }
            }
        }

        info!("All permissions requested successfully");
        Ok(())
    }

    /// 個別の権限を要求
    pub async fn request_permission(&self, permission: Permission) -> KvmResult<PermissionStatus> {
        match permission {
            Permission::ScreenCapture => self.request_screen_capture().await,
            Permission::InputDevices => self.request_input_devices().await,
            Permission::NetworkAccess => self.request_network_access().await,
            Permission::FileSystemAccess => self.request_filesystem_access().await,
            Permission::SystemService => self.request_system_service().await,
        }
    }

    /// 権限が付与されているか確認
    pub fn is_granted(&self, permission: &Permission) -> bool {
        self.granted_permissions.contains(permission)
    }

    /// 全権限が付与されているか確認
    pub fn all_granted(&self) -> bool {
        self.required_permissions
            .iter()
            .all(|p| self.is_granted(p))
    }

    /// 画面キャプチャ権限を要求（プラットフォーム固有）
    #[cfg(target_os = "linux")]
    async fn request_screen_capture(&self) -> KvmResult<PermissionStatus> {
        crate::linux::wayland::request_screen_capture_permission().await
    }

    #[cfg(target_os = "macos")]
    async fn request_screen_capture(&self) -> KvmResult<PermissionStatus> {
        crate::macos::request_screen_capture_permission().await
    }

    #[cfg(target_os = "windows")]
    async fn request_screen_capture(&self) -> KvmResult<PermissionStatus> {
        crate::windows::request_screen_capture_permission().await
    }

    /// 入力デバイス権限を要求
    async fn request_input_devices(&self) -> KvmResult<PermissionStatus> {
        // 通常のアプリケーションでは特別な権限は不要
        // ただし、キーロガー等の特殊なケースでは追加の権限が必要
        Ok(PermissionStatus::Granted)
    }

    /// ネットワークアクセス権限を要求
    async fn request_network_access(&self) -> KvmResult<PermissionStatus> {
        // LAN専用なので通常のネットワーク権限で十分
        Ok(PermissionStatus::Granted)
    }

    /// ファイルシステムアクセス権限を要求
    async fn request_filesystem_access(&self) -> KvmResult<PermissionStatus> {
        // 設定ファイルとログ用の権限
        Ok(PermissionStatus::Granted)
    }

    /// システムサービス権限を要求
    async fn request_system_service(&self) -> KvmResult<PermissionStatus> {
        // サービスインストール時の権限
        #[cfg(unix)]
        {
            // root権限が必要な場合
            if !is_root() {
                warn!("System service installation requires root privileges");
                return Ok(PermissionStatus::Denied);
            }
        }

        Ok(PermissionStatus::Granted)
    }
}

/// Linux Wayland 権限要求実装
#[cfg(target_os = "linux")]
pub mod wayland_permissions {
    use super::*;
    use std::process::Command;
    use tokio::process::Command as TokioCommand;

    /// Wayland 画面キャプチャ権限を要求
    pub async fn request_screen_capture_permission() -> KvmResult<PermissionStatus> {
        info!("Requesting Wayland screen capture permission...");

        // xdg-desktop-portal を使用して権限を要求
        // 実際の実装では、適切なportalプロトコルを使用

        // 1. 利用可能なポータルを確認
        if !is_portal_available() {
            warn!("xdg-desktop-portal not available, falling back to direct access");
            return Ok(PermissionStatus::NotApplicable);
        }

        // 2. スクリーンキャストポータルの利用を要求
        match request_screencast_portal().await {
            Ok(()) => {
                info!("Wayland screen capture permission granted via portal");
                Ok(PermissionStatus::Granted)
            }
            Err(e) => {
                error!("Failed to request screen capture permission: {}", e);

                // フォールバック: 直接アクセスを試行
                match try_direct_access().await {
                    Ok(()) => {
                        info!("Direct screen capture access granted");
                        Ok(PermissionStatus::Granted)
                    }
                    Err(_) => {
                        error!("Both portal and direct access failed");
                        Ok(PermissionStatus::Denied)
                    }
                }
            }
        }
    }

    /// xdg-desktop-portal が利用可能か確認
    fn is_portal_available() -> bool {
        std::env::var("XDG_CURRENT_DESKTOP").is_ok()
            && Command::new("busctl")
                .args(["--user", "call", "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.ScreenCast", "CreateSession", "a{sv}", "0"])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    /// スクリーンキャストポータルを要求
    async fn request_screencast_portal() -> KvmResult<()> {
        // 実際の実装では、D-Bus経由でポータルに要求
        // ここでは簡易的な実装としてコマンド実行

        let output = TokioCommand::new("gdbus")
            .args([
                "call", "--session",
                "--dest", "org.freedesktop.portal.Desktop",
                "--object-path", "/org/freedesktop/portal/desktop",
                "--method", "org.freedesktop.portal.ScreenCast.CreateSession",
                "{}"
            ])
            .output()
            .await
            .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Portal request failed: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(soft_kvm_core::KvmError::Platform(format!("Portal denied access: {}", stderr)))
        }
    }

    /// 直接アクセスを試行（X11や古いWayland）
    async fn try_direct_access() -> KvmResult<()> {
        // 環境変数で権限が付与されているか確認
        if std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "wayland" {
            // Waylandの場合、wlrootsベースのコンポジタでは追加の設定が必要
            warn!("Wayland direct access requires compositor-specific configuration");
            return Err(soft_kvm_core::KvmError::Platform("Direct Wayland access not configured".to_string()));
        }

        // X11の場合
        if std::env::var("DISPLAY").is_ok() {
            // X11では通常のアクセス権限で十分
            Ok(())
        } else {
            Err(soft_kvm_core::KvmError::Platform("No display server available".to_string()))
        }
    }

    /// Wayland デバッグ権限を設定
    pub async fn setup_wayland_debug() -> KvmResult<()> {
        // WAYLAND_DEBUG 環境変数を設定
        std::env::set_var("WAYLAND_DEBUG", "1");

        // wlroots デバッグ出力
        std::env::set_var("WLR_DEBUG", "all");

        info!("Wayland debug logging enabled");
        Ok(())
    }
}

/// 一般的な権限ユーティリティ
pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// 設定ディレクトリのパスを取得
pub fn get_config_dir() -> KvmResult<PathBuf> {
    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| soft_kvm_core::KvmError::Platform("Failed to get base directories".to_string()))?;

    #[cfg(unix)]
    {
        Ok(base_dirs.config_dir().join("soft-kvm"))
    }
    #[cfg(windows)]
    {
        Ok(base_dirs.config_dir().join("Soft KVM"))
    }
}

/// ログディレクトリのパスを取得
pub fn get_log_dir() -> KvmResult<PathBuf> {
    #[cfg(unix)]
    {
        if is_root() {
            Ok(PathBuf::from("/var/log/soft-kvm"))
        } else {
            let base_dirs = directories::BaseDirs::new()
                .ok_or_else(|| soft_kvm_core::KvmError::Platform("Failed to get base directories".to_string()))?;
            Ok(base_dirs.data_local_dir().join("soft-kvm").join("logs"))
        }
    }
    #[cfg(windows)]
    {
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| soft_kvm_core::KvmError::Platform("Failed to get base directories".to_string()))?;
        Ok(base_dirs.data_local_dir().join("Soft KVM").join("logs"))
    }
}

#[cfg(not(target_os = "linux"))]
pub mod wayland_permissions {
    use super::*;

    pub async fn request_screen_capture_permission() -> KvmResult<PermissionStatus> {
        // 非Linuxプラットフォームでは該当なし
        Ok(PermissionStatus::NotApplicable)
    }

    pub async fn setup_wayland_debug() -> KvmResult<()> {
        // 非Linuxプラットフォームでは該当なし
        Ok(())
    }
}
