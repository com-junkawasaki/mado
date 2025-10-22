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

//! # Soft KVM Discovery
//!
//! Service discovery functionality for Soft KVM

use soft_kvm_core::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;
use tracing::{debug, info, warn, error};
use std::net::IpAddr;
use serde::{Serialize, Deserialize};

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: ServiceId,
    pub name: String,
    pub service_type: ServiceType,
    pub address: NetworkAddress,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

impl ServiceInfo {
    /// Check if the service is expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now();
        let elapsed = now.signed_duration_since(self.last_seen);
        elapsed > chrono::Duration::seconds(300) // 5 minutes
    }
}

/// Service resolver for mDNS discovery
pub struct ServiceResolver {
    services: Arc<RwLock<HashMap<ServiceId, ServiceInfo>>>,
    service_type: ServiceType,
    discovery_handle: Arc<RwLock<Option<task::JoinHandle<()>>>>,
    registration_handle: Arc<RwLock<Option<task::JoinHandle<()>>>>,
}

impl ServiceResolver {
    /// Create a new service resolver
    pub fn new(service_type: ServiceType) -> Self {
        ServiceResolver {
            services: Arc::new(RwLock::new(HashMap::new())),
            service_type,
            discovery_handle: Arc::new(RwLock::new(None)),
            registration_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start service discovery
    pub async fn start_discovery(&self) -> KvmResult<()> {
        info!("Starting mDNS service discovery for {:?}", self.service_type);

        // すでに実行中の場合は何もしない
        let mut discovery_handle = self.discovery_handle.write().await;
        if discovery_handle.is_some() {
            debug!("Service discovery already running");
            return Ok(());
        }

        // mDNSサービス名の設定
        let service_name = match self.service_type {
            ServiceType::Server => "_soft-kvm-server._tcp.local.",
            ServiceType::Client => "_soft-kvm-client._tcp.local.",
        };

        let services_clone = Arc::clone(&self.services);

        // mDNS discovery taskを開始
        let handle = task::spawn(async move {
            debug!("Starting mDNS discovery for service: {}", service_name);

            // 実際のmDNS実装は簡易的なものに置き換え
            // 本番実装では適切なmDNSライブラリを使用
            info!("mDNS discovery placeholder - would discover services via {}", service_name);

            // 定期的なクリーンアップタスク
            let cleanup_services = Arc::clone(&services_clone);
            let cleanup_handle = task::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    Self::cleanup_expired_services(&cleanup_services).await;
                }
            });

            // ダミーのサービス検出（テスト用）
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            info!("mDNS discovery simulation completed");
        });

        *discovery_handle = Some(handle);

        // 定期クリーンアップを開始
        let services_clone = Arc::clone(&self.services);
        let cleanup_handle = task::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                Self::cleanup_expired_services(&services_clone).await;
            }
        });

        Ok(())
    }


    /// Clean up expired services
    async fn cleanup_expired_services(services: &Arc<RwLock<HashMap<ServiceId, ServiceInfo>>>) {
        debug!("Cleaning up expired services");

        let mut services_map = services.write().await;
        let expired_ids: Vec<ServiceId> = services_map
            .iter()
            .filter(|(_, info)| info.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            services_map.remove(&id);
            debug!("Removed expired service: {:?}", id);
        }

        if !services_map.is_empty() {
            debug!("Active services: {}", services_map.len());
        }
    }

    /// Stop service discovery
    pub async fn stop_discovery(&self) -> KvmResult<()> {
        info!("Stopping service discovery");

        let mut discovery_handle = self.discovery_handle.write().await;
        if let Some(handle) = discovery_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Get available services
    pub async fn get_available_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services.values()
            .filter(|service| !service.is_expired())
            .cloned()
            .collect()
    }

    /// Register a service and advertise it via mDNS
    pub async fn register_service(&self, info: ServiceInfo) -> KvmResult<()> {
        info!("Registering service: {} ({:?})", info.name, info.service_type);

        // ローカルサービスマップに追加
        {
            let mut services = self.services.write().await;
            services.insert(info.id.clone(), info.clone());
        }

        // すでに登録中の場合は何もしない
        let mut registration_handle = self.registration_handle.write().await;
        if registration_handle.is_some() {
            debug!("Service already registered");
            return Ok(());
        }

        // mDNSサービス名を設定
        let service_type_name = match info.service_type {
            ServiceType::Server => "_soft-kvm-server._tcp",
            ServiceType::Client => "_soft-kvm-client._tcp",
        };

        let hostname = info.name.clone();
        let port = info.address.port;
        let service_name = format!("{}.{}", hostname, service_type_name);

        let services_clone = Arc::clone(&self.services);

        // mDNS registration taskを開始
        let handle = task::spawn(async move {
            debug!("Starting mDNS registration for service: {}", service_name);

            // 実際のmDNS実装は簡易的なものに置き換え
            // 本番実装では適切なmDNSライブラリを使用
            info!("mDNS registration placeholder - would register service {} at {}:{}", hostname, hostname, port);

            // 定期的にサービスを更新（TTLを維持）
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5分毎
            loop {
                interval.tick().await;

                // サービスがまだ有効かチェック
                let services = services_clone.read().await;
                if !services.contains_key(&info.id) {
                    debug!("Service no longer registered, stopping mDNS advertisement");
                    break;
                }

                debug!("Refreshing mDNS registration for {}", service_name);
            }
        });

        *registration_handle = Some(handle);

        Ok(())
    }

    /// Unregister a service and stop mDNS advertisement
    pub async fn unregister_service(&self, id: &ServiceId) -> KvmResult<()> {
        info!("Unregistering service: {:?}", id);

        // ローカルサービスマップから削除
        {
            let mut services = self.services.write().await;
            services.remove(id);
        }

        // mDNS登録タスクを停止
        let mut registration_handle = self.registration_handle.write().await;
        if let Some(handle) = registration_handle.take() {
            handle.abort();
            debug!("Stopped mDNS registration task");
        }

        Ok(())
    }
}
