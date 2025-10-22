//! Linux systemd サービス実装

use soft_kvm_core::KvmResult;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, info};

/// systemd サービスファイルテンプレート
pub const SYSTEMD_SERVICE_TEMPLATE: &str = r#"[Unit]
Description=Soft KVM Service - LAN専用・低遅延KVM共有システム
After=network.target
Wants=network.target

[Service]
Type=simple
User=soft-kvm
Group=soft-kvm
ExecStart=/usr/local/bin/soft-kvm-server
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=soft-kvm

# セキュリティ設定
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/soft-kvm /var/log/soft-kvm
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# 機能制限
MemoryLimit=256M
CPUQuota=50%
BlockIOWeight=10

# Wayland/X11 アクセス
Environment=DISPLAY=:0
Environment=XAUTHORITY=/home/soft-kvm/.Xauthority
Environment=WAYLAND_DISPLAY=wayland-0

[Install]
WantedBy=multi-user.target
"#;

/// systemd サービスを開始
pub async fn start_service() -> KvmResult<()> {
    info!("Starting systemd service...");

    // サービスファイルが存在するか確認
    if !service_file_exists() {
        create_service_file().await?;
        reload_daemon().await?;
    }

    // サービスを開始
    let output = TokioCommand::new("systemctl")
        .args(["start", "soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to start service: {}", e)))?;

    if output.status.success() {
        info!("systemd service started successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to start systemd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemd start failed: {}", stderr)))
    }
}

/// systemd サービスを停止
pub async fn stop_service() -> KvmResult<()> {
    info!("Stopping systemd service...");

    let output = TokioCommand::new("systemctl")
        .args(["stop", "soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to stop service: {}", e)))?;

    if output.status.success() {
        info!("systemd service stopped successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to stop systemd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemd stop failed: {}", stderr)))
    }
}

/// systemd サービスを再起動
pub async fn restart_service() -> KvmResult<()> {
    info!("Restarting systemd service...");

    let output = TokioCommand::new("systemctl")
        .args(["restart", "soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to restart service: {}", e)))?;

    if output.status.success() {
        info!("systemd service restarted successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to restart systemd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemd restart failed: {}", stderr)))
    }
}

/// サービス状態を取得
pub async fn get_service_status() -> KvmResult<String> {
    let output = TokioCommand::new("systemctl")
        .args(["status", "soft-kvm.service", "--no-pager", "-l"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to get status: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemctl status failed: {}", stderr)))
    }
}

/// サービスファイルが存在するか確認
fn service_file_exists() -> bool {
    std::path::Path::new("/etc/systemd/system/soft-kvm.service").exists()
}

/// systemd サービスファイルを作成
async fn create_service_file() -> KvmResult<()> {
    info!("Creating systemd service file...");

    // サービスファイルを作成
    tokio::fs::write("/etc/systemd/system/soft-kvm.service", SYSTEMD_SERVICE_TEMPLATE)
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to create service file: {}", e)))?;

    // 実行権限を設定
    TokioCommand::new("chmod")
        .args(["644", "/etc/systemd/system/soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to set permissions: {}", e)))?;

    info!("systemd service file created successfully");
    Ok(())
}

/// systemd daemon をリロード
async fn reload_daemon() -> KvmResult<()> {
    info!("Reloading systemd daemon...");

    let output = TokioCommand::new("systemctl")
        .arg("daemon-reload")
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to reload daemon: {}", e)))?;

    if output.status.success() {
        info!("systemd daemon reloaded successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to reload systemd daemon: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("daemon-reload failed: {}", stderr)))
    }
}

/// サービスを有効化（自動起動）
pub async fn enable_service() -> KvmResult<()> {
    info!("Enabling systemd service...");

    let output = TokioCommand::new("systemctl")
        .args(["enable", "soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to enable service: {}", e)))?;

    if output.status.success() {
        info!("systemd service enabled successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to enable systemd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemctl enable failed: {}", stderr)))
    }
}

/// サービスを無効化
pub async fn disable_service() -> KvmResult<()> {
    info!("Disabling systemd service...");

    let output = TokioCommand::new("systemctl")
        .args(["disable", "soft-kvm.service"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to disable service: {}", e)))?;

    if output.status.success() {
        info!("systemd service disabled successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to disable systemd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("systemctl disable failed: {}", stderr)))
    }
}
