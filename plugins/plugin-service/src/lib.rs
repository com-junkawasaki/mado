// Copyright 2024 Soft KVM Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Soft KVM Service Plugin
//!
//! Tauri plugin for system service management

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Import modules
mod systemd;
#[cfg(target_os = "macos")]
mod launchd;
#[cfg(target_os = "windows")]
mod windows_svc;
mod config;

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleServiceConfig {
    pub service_name: String,
    pub auto_start: bool,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: bool,
    pub auto_start: bool,
}

#[derive(Debug)]
struct ServiceState {
    config: Option<SimpleServiceConfig>,
    is_running: bool,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState {
            config: None,
            is_running: false,
        }
    }
}

/// Install system service
#[tauri::command]
async fn install_service(
    config: SimpleServiceConfig,
    state: tauri::State<'_, Arc<RwLock<ServiceState>>>,
) -> Result<String, String> {
    println!("Installing service: {:?}", config);

    let mut service_state = state.write().await;

    // Platform-specific installation
    #[cfg(target_os = "linux")]
    {
        match systemd::install_service(Some(&config)).await {
            Ok(_) => {
                service_state.config = Some(config);
                Ok("Systemd service installed successfully".to_string())
            }
            Err(e) => Err(format!("Failed to install systemd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match launchd::install_service(Some(&config)).await {
            Ok(_) => {
                service_state.config = Some(config);
                Ok("Launchd service installed successfully".to_string())
            }
            Err(e) => Err(format!("Failed to install launchd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match windows_svc::install_service().await {
            Ok(_) => {
                service_state.config = Some(config);
                Ok("Windows service installed successfully".to_string())
            }
            Err(e) => Err(format!("Failed to install Windows service: {:?}", e)),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Service installation not supported on this platform".to_string())
    }
}

/// Uninstall system service
#[tauri::command]
async fn uninstall_service(
    service_name: String,
    state: tauri::State<'_, Arc<RwLock<ServiceState>>>,
) -> Result<String, String> {
    println!("Uninstalling service: {}", service_name);

    let mut service_state = state.write().await;

    // Platform-specific uninstallation
    #[cfg(target_os = "linux")]
    {
        match systemd::uninstall_service().await {
            Ok(_) => {
                service_state.config = None;
                service_state.is_running = false;
                Ok("Systemd service uninstalled successfully".to_string())
            }
            Err(e) => Err(format!("Failed to uninstall systemd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match launchd::uninstall_service().await {
            Ok(_) => {
                service_state.config = None;
                service_state.is_running = false;
                Ok("Launchd service uninstalled successfully".to_string())
            }
            Err(e) => Err(format!("Failed to uninstall launchd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match windows_svc::uninstall_service().await {
            Ok(_) => {
                service_state.config = None;
                service_state.is_running = false;
                Ok("Windows service uninstalled successfully".to_string())
            }
            Err(e) => Err(format!("Failed to uninstall Windows service: {:?}", e)),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Service uninstallation not supported on this platform".to_string())
    }
}

/// Start system service
#[tauri::command]
async fn start_service(
    service_name: String,
    state: tauri::State<'_, Arc<RwLock<ServiceState>>>,
) -> Result<String, String> {
    println!("Starting service: {}", service_name);

    let mut service_state = state.write().await;

    // Platform-specific start
    #[cfg(target_os = "linux")]
    {
        match systemd::start_service().await {
            Ok(_) => {
                service_state.is_running = true;
                Ok("Systemd service started successfully".to_string())
            }
            Err(e) => Err(format!("Failed to start systemd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match launchd::start_service().await {
            Ok(_) => {
                service_state.is_running = true;
                Ok("Launchd service started successfully".to_string())
            }
            Err(e) => Err(format!("Failed to start launchd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match windows_svc::start_service().await {
            Ok(_) => {
                service_state.is_running = true;
                Ok("Windows service started successfully".to_string())
            }
            Err(e) => Err(format!("Failed to start Windows service: {:?}", e)),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Service start not supported on this platform".to_string())
    }
}

/// Stop system service
#[tauri::command]
async fn stop_service(
    service_name: String,
    state: tauri::State<'_, Arc<RwLock<ServiceState>>>,
) -> Result<String, String> {
    println!("Stopping service: {}", service_name);

    let mut service_state = state.write().await;

    // Platform-specific stop
    #[cfg(target_os = "linux")]
    {
        match systemd::stop_service().await {
            Ok(_) => {
                service_state.is_running = false;
                Ok("Systemd service stopped successfully".to_string())
            }
            Err(e) => Err(format!("Failed to stop systemd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match launchd::stop_service().await {
            Ok(_) => {
                service_state.is_running = false;
                Ok("Launchd service stopped successfully".to_string())
            }
            Err(e) => Err(format!("Failed to stop launchd service: {:?}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match windows_svc::stop_service().await {
            Ok(_) => {
                service_state.is_running = false;
                Ok("Windows service stopped successfully".to_string())
            }
            Err(e) => Err(format!("Failed to stop Windows service: {:?}", e)),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Service stop not supported on this platform".to_string())
    }
}

/// Get service status
#[tauri::command]
async fn get_service_status(
    service_name: String,
    state: tauri::State<'_, Arc<RwLock<ServiceState>>>,
) -> Result<ServiceStatus, String> {
    println!("Getting status for service: {}", service_name);

    let service_state = state.read().await;

    // Platform-specific status check
    let running = {
        #[cfg(target_os = "linux")]
        {
            systemd::get_service_status().await.is_ok()
        }
        #[cfg(target_os = "macos")]
        {
            launchd::get_service_status().await.is_ok()
        }
        #[cfg(target_os = "windows")]
        {
            windows_svc::get_service_status().await.is_ok()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    };

    let installed = service_state.config.is_some();
    let auto_start = service_state.config.as_ref().map(|c| c.auto_start).unwrap_or(false);

    Ok(ServiceStatus {
        installed,
        running: running && service_state.is_running,
        auto_start,
    })
}

/// Initialize the service plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-service")
        .invoke_handler(tauri::generate_handler![
            install_service,
            uninstall_service,
            start_service,
            stop_service,
            get_service_status,
        ])
        .setup(|app, _app_handle| {
            // Initialize service state
            let state = Arc::new(RwLock::new(ServiceState::default()));
            app.manage(state);
            Ok(())
        })
        .build()
}
