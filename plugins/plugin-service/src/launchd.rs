//! macOS launchd サービス実装

use soft_kvm_core::KvmResult;
use std::process::Command;
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, info, warn};

/// launchd プロパティリストテンプレート
pub const LAUNCHD_PLIST_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.soft-kvm.server</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/soft-kvm-server</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/var/log/soft-kvm.log</string>

    <key>StandardErrorPath</key>
    <string>/var/log/soft-kvm.error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
        <key>DISPLAY</key>
        <string>:0</string>
    </dict>

    <key>UserName</key>
    <string>root</string>

    <key>GroupName</key>
    <string>wheel</string>

    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>1024</integer>
    </dict>

    <key>HardResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>2048</integer>
    </dict>
</dict>
</plist>
"#;

/// launchd サービスを開始
pub async fn start_service() -> KvmResult<()> {
    info!("Starting launchd service...");

    // plistファイルが存在するか確認
    if !plist_file_exists() {
        create_plist_file(None).await?;
        load_service().await?;
    }

    // サービスを開始（すでにロードされている場合はスキップ）
    let output = TokioCommand::new("launchctl")
        .args(["start", "com.soft-kvm.server"])
        .output()
        .await;

    match output {
        Ok(result) if result.status.success() => {
            info!("launchd service started successfully");
            Ok(())
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            // すでに実行中の場合は成功として扱う
            if stderr.contains("already running") {
                info!("launchd service is already running");
                Ok(())
            } else {
                error!("Failed to start launchd service: {}", stderr);
                Err(soft_kvm_core::KvmError::Platform(format!("launchctl start failed: {}", stderr)))
            }
        }
        Err(e) => {
            error!("Failed to execute launchctl start: {}", e);
            Err(soft_kvm_core::KvmError::Platform(format!("launchctl start command failed: {}", e)))
        }
    }
}

/// launchd サービスを停止
pub async fn stop_service() -> KvmResult<()> {
    info!("Stopping launchd service...");

    let output = TokioCommand::new("launchctl")
        .args(["stop", "com.soft-kvm.server"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to stop service: {}", e)))?;

    if output.status.success() {
        info!("launchd service stopped successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to stop launchd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("launchctl stop failed: {}", stderr)))
    }
}

/// launchd サービスを再起動
pub async fn restart_service() -> KvmResult<()> {
    info!("Restarting launchd service...");

    stop_service().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    start_service().await
}

/// サービス状態を取得
pub async fn get_service_status() -> KvmResult<String> {
    let output = TokioCommand::new("launchctl")
        .args(["list", "com.soft-kvm.server"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to get status: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("launchctl list failed: {}", stderr)))
    }
}

/// plistファイルが存在するか確認
fn plist_file_exists() -> bool {
    std::path::Path::new("/Library/LaunchDaemons/com.soft-kvm.server.plist").exists()
}

/// launchd plistファイルを作成
async fn create_plist_file(config: Option<&crate::SimpleServiceConfig>) -> KvmResult<()> {
    info!("Creating launchd plist file...");

    let plist_content = if let Some(config) = config {
        // 設定に基づいてplistをカスタマイズ
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.soft-kvm.server</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/soft-kvm-server</string>
    </array>

    <key>RunAtLoad</key>
    <{}/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/var/log/soft-kvm.log</string>

    <key>StandardErrorPath</key>
    <string>/var/log/soft-kvm.error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
        <key>DISPLAY</key>
        <string>:0</string>
    </dict>

    <key>UserName</key>
    <string>root</string>

    <key>GroupName</key>
    <string>wheel</string>

    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>1024</integer>
    </dict>

    <key>HardResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>2048</integer>
    </dict>
</dict>
</plist>
"#, if config.auto_start { "true" } else { "false" })
    } else {
        LAUNCHD_PLIST_TEMPLATE.to_string()
    };

    // plistファイルを作成
    tokio::fs::write("/Library/LaunchDaemons/com.soft-kvm.server.plist", plist_content)
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to create plist file: {}", e)))?;

    // 適切な権限を設定
    TokioCommand::new("chmod")
        .args(["644", "/Library/LaunchDaemons/com.soft-kvm.server.plist"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to set permissions: {}", e)))?;

    // 所有者をrootに設定
    TokioCommand::new("chown")
        .args(["root:wheel", "/Library/LaunchDaemons/com.soft-kvm.server.plist"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to set ownership: {}", e)))?;

    info!("launchd plist file created successfully");
    Ok(())
}

/// launchdサービスをロード
async fn load_service() -> KvmResult<()> {
    info!("Loading launchd service...");

    let output = TokioCommand::new("launchctl")
        .args(["load", "/Library/LaunchDaemons/com.soft-kvm.server.plist"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to load service: {}", e)))?;

    if output.status.success() {
        info!("launchd service loaded successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to load launchd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("launchctl load failed: {}", stderr)))
    }
}

/// launchdサービスをインストール
pub async fn install_service(config: Option<&crate::SimpleServiceConfig>) -> KvmResult<()> {
    info!("Installing launchd service...");

    // plistファイルを作成
    create_plist_file(config).await?;

    // サービスをロード
    load_service().await
}

/// launchdサービスをアンインストール
pub async fn uninstall_service() -> KvmResult<()> {
    info!("Uninstalling launchd service...");

    // サービスをアンロード
    if let Err(e) = unload_service().await {
        warn!("Failed to unload service during uninstall: {:?}", e);
    }

    // plistファイルを削除
    match std::fs::remove_file("/Library/LaunchDaemons/com.soft-kvm.server.plist") {
        Ok(_) => {
            info!("launchd plist file removed successfully");
            info!("launchd service uninstalled successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to remove plist file: {}", e);
            Err(soft_kvm_core::KvmError::Platform(format!("Failed to remove plist file: {}", e)))
        }
    }
}

/// launchdサービスをアンロード
pub async fn unload_service() -> KvmResult<()> {
    info!("Unloading launchd service...");

    let output = TokioCommand::new("launchctl")
        .args(["unload", "/Library/LaunchDaemons/com.soft-kvm.server.plist"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to unload service: {}", e)))?;

    if output.status.success() {
        info!("launchd service unloaded successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to unload launchd service: {}", stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("launchctl unload failed: {}", stderr)))
    }
}

/// サービスログを表示
pub async fn show_logs() -> KvmResult<()> {
    let output = TokioCommand::new("tail")
        .args(["-f", "-n", "50", "/var/log/soft-kvm.log"])
        .output()
        .await
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to show logs: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(soft_kvm_core::KvmError::Platform(format!("tail command failed: {}", stderr)))
    }
}
