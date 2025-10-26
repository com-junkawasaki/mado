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

//! # Soft KVM Discovery Plugin
//!
//! Tauri plugin for service discovery using mDNS

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use soft_kvm_discovery::ServiceResolver;
use soft_kvm_core::{ServiceId, ServiceType, NetworkAddress};
use soft_kvm_discovery::ServiceInfo;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscoveryConfig {
    pub service_type: ServiceType,
    pub auto_discovery: bool,
    pub discovery_interval: u64, // seconds
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscoveryStatus {
    pub is_discovering: bool,
    pub is_registered: bool,
    pub available_services: Vec<ServiceInfo>,
    pub registered_service: Option<ServiceInfo>,
}

struct DiscoveryState {
    resolver: Option<ServiceResolver>,
    config: Option<DiscoveryConfig>,
    registered_service: Option<ServiceInfo>,
}

impl Default for DiscoveryState {
    fn default() -> Self {
        DiscoveryState {
            resolver: None,
            config: None,
            registered_service: None,
        }
    }
}

/// Initialize discovery
#[tauri::command]
async fn init_discovery(
    config: DiscoveryConfig,
    state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>,
) -> Result<String, String> {
    println!("Initializing discovery with config: {:?}", config);

    let mut discovery_state = state.write().await;

    // サービスリゾルバを作成
    let resolver = ServiceResolver::new(config.service_type.clone());

    // 自動発見が有効な場合は開始
    if config.auto_discovery {
        if let Err(e) = resolver.start_discovery().await {
            return Err(format!("Failed to start discovery: {}", e));
        }
    }

    discovery_state.resolver = Some(resolver);
    discovery_state.config = Some(config);

    Ok("Discovery initialized successfully".to_string())
}

/// Start service discovery
#[tauri::command]
async fn start_discovery(state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>) -> Result<String, String> {
    let discovery_state = state.read().await;

    if let Some(resolver) = &discovery_state.resolver {
        resolver.start_discovery().await
            .map_err(|e| format!("Failed to start discovery: {}", e))?;
        Ok("Service discovery started".to_string())
    } else {
        Err("Discovery not initialized".to_string())
    }
}

/// Stop service discovery
#[tauri::command]
async fn stop_discovery(state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>) -> Result<String, String> {
    let discovery_state = state.read().await;

    if let Some(resolver) = &discovery_state.resolver {
        resolver.stop_discovery().await
            .map_err(|e| format!("Failed to stop discovery: {}", e))?;
        Ok("Service discovery stopped".to_string())
    } else {
        Err("Discovery not initialized".to_string())
    }
}

/// Register local service
#[tauri::command]
async fn register_service(
    service_name: String,
    service_type: ServiceType,
    address: NetworkAddress,
    state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>,
) -> Result<String, String> {
    let mut discovery_state = state.write().await;

    if let Some(resolver) = &discovery_state.resolver {
        // サービスIDを生成
        let service_id = ServiceId(uuid::Uuid::new_v4());

        let service_info = ServiceInfo {
            id: service_id,
            name: service_name.clone(),
            service_type: service_type.clone(),
            address: address.clone(),
            last_seen: chrono::Utc::now(),
        };

        resolver.register_service(service_info.clone()).await
            .map_err(|e| format!("Failed to register service: {}", e))?;

        discovery_state.registered_service = Some(service_info);

        Ok(format!("Service registered: {} ({:?}) at {}:{}", service_name, service_type, address.ip, address.port))
    } else {
        Err("Discovery not initialized".to_string())
    }
}

/// Unregister local service
#[tauri::command]
async fn unregister_service(state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>) -> Result<String, String> {
    let mut discovery_state = state.write().await;

    if let Some(service_info) = &discovery_state.registered_service {
        if let Some(resolver) = &discovery_state.resolver {
            resolver.unregister_service(&service_info.id).await
                .map_err(|e| format!("Failed to unregister service: {}", e))?;

            discovery_state.registered_service = None;

            Ok("Service unregistered".to_string())
        } else {
            Err("Discovery not initialized".to_string())
        }
    } else {
        Err("No service registered".to_string())
    }
}

/// Get available services
#[tauri::command]
async fn get_available_services(state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>) -> Result<Vec<ServiceInfo>, String> {
    let discovery_state = state.read().await;

    if let Some(resolver) = &discovery_state.resolver {
        let services = resolver.get_available_services().await;
        Ok(services)
    } else {
        Err("Discovery not initialized".to_string())
    }
}

/// Get discovery status
#[tauri::command]
async fn get_discovery_status(state: tauri::State<'_, Arc<RwLock<DiscoveryState>>>) -> Result<DiscoveryStatus, String> {
    let discovery_state = state.read().await;

    let is_discovering = discovery_state.resolver.is_some();
    let is_registered = discovery_state.registered_service.is_some();

    let available_services = if let Some(resolver) = &discovery_state.resolver {
        resolver.get_available_services().await
    } else {
        Vec::new()
    };

    Ok(DiscoveryStatus {
        is_discovering,
        is_registered,
        available_services,
        registered_service: discovery_state.registered_service.clone(),
    })
}

/// Initialize the discovery plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-discovery")
        .invoke_handler(tauri::generate_handler![
            init_discovery,
            start_discovery,
            stop_discovery,
            register_service,
            unregister_service,
            get_available_services,
            get_discovery_status,
        ])
        .setup(|app, _app_handle| {
            // Initialize discovery state
            let state = Arc::new(RwLock::new(DiscoveryState::default()));
            app.manage(state);
            Ok(())
        })
        .build()
}
