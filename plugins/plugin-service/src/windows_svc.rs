//! Windows サービス実装

use soft_kvm_core::KvmResult;
use std::ffi::OsString;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

/// Windows サービス名
pub const SERVICE_NAME: &str = "soft-kvm";
pub const SERVICE_DISPLAY_NAME: &str = "Soft KVM Service";
pub const SERVICE_DESCRIPTION: &str = "LAN専用・低遅延KVM共有システム";

/// サービスメイン関数
pub fn service_main() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

/// FFI サービスメイン関数
define_windows_service!(ffi_service_main, service_main_func);

/// サービスメイン関数実装
pub fn service_main_func(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        error!("Service failed: {:?}", e);
    }
}

/// サービス実行関数
fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Received stop signal");
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(next_status)?;

    info!("Soft KVM Windows service started");

    // メインサービスループ
    loop {
        time::sleep(Duration::from_secs(1)).await;

        // 停止シグナルが来たら終了
        match status_handle.wait_for_service_stop(Duration::from_secs(1)) {
            Ok(()) => break,
            Err(_) => continue,
        }
    }

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(next_status)?;

    info!("Soft KVM Windows service stopped");
    Ok(())
}

/// Windows サービスを開始（外部からの呼び出し用）
pub async fn start_service() -> KvmResult<()> {
    info!("Starting Windows service...");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::START;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to connect to service manager: {}", e)))?;

    let service = service_manager
        .open_service(SERVICE_NAME, windows_service::service::ServiceAccess::START)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to open service: {}", e)))?;

    service
        .start(&[OsString::default()])
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to start service: {}", e)))?;

    info!("Windows service started successfully");
    Ok(())
}

/// Windows サービスを停止
pub async fn stop_service() -> KvmResult<()> {
    info!("Stopping Windows service...");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::STOP;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to connect to service manager: {}", e)))?;

    let service = service_manager
        .open_service(SERVICE_NAME, windows_service::service::ServiceAccess::STOP)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to open service: {}", e)))?;

    service
        .stop()
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to stop service: {}", e)))?;

    info!("Windows service stopped successfully");
    Ok(())
}

/// Windows サービスをインストール
pub async fn install_service() -> KvmResult<()> {
    info!("Installing Windows service...");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to connect to service manager: {}", e)))?;

    let service_info = windows_service::service::ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: windows_service::service::ServiceStartType::AutoStart,
        error_control: windows_service::service::ServiceErrorControl::Normal,
        executable_path: std::env::current_exe()
            .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to get executable path: {}", e)))?,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = service_manager
        .create_service(&service_info, windows_service::service::ServiceAccess::empty())
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to create service: {}", e)))?;

    service
        .set_description(SERVICE_DESCRIPTION)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to set description: {}", e)))?;

    info!("Windows service installed successfully");
    Ok(())
}

/// Windows サービスをアンインストール
pub async fn uninstall_service() -> KvmResult<()> {
    info!("Uninstalling Windows service...");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::DELETE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to connect to service manager: {}", e)))?;

    let service = service_manager
        .open_service(SERVICE_NAME, windows_service::service::ServiceAccess::DELETE)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to open service: {}", e)))?;

    service
        .delete()
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to delete service: {}", e)))?;

    info!("Windows service uninstalled successfully");
    Ok(())
}

/// サービス状態を取得
pub async fn get_service_status() -> KvmResult<String> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to connect to service manager: {}", e)))?;

    let service = service_manager
        .open_service(SERVICE_NAME, windows_service::service::ServiceAccess::QUERY_STATUS)
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to open service: {}", e)))?;

    let status = service
        .query_status()
        .map_err(|e| soft_kvm_core::KvmError::Platform(format!("Failed to query status: {}", e)))?;

    let status_str = match status.current_state {
        ServiceState::Stopped => "Stopped",
        ServiceState::StartPending => "Start Pending",
        ServiceState::StopPending => "Stop Pending",
        ServiceState::Running => "Running",
        ServiceState::ContinuePending => "Continue Pending",
        ServiceState::PausePending => "Pause Pending",
        ServiceState::Paused => "Paused",
    };

    Ok(format!("Service Status: {}", status_str))
}

/// サービスが存在するか確認
pub async fn service_exists() -> bool {
    let manager_access = ServiceManagerAccess::CONNECT;
    if let Ok(service_manager) = ServiceManager::local_computer(None::<&str>, manager_access) {
        service_manager
            .open_service(SERVICE_NAME, windows_service::service::ServiceAccess::QUERY_CONFIG)
            .is_ok()
    } else {
        false
    }
}
