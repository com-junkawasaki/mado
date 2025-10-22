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

/// Service information
#[derive(Debug, Clone)]
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
}

impl ServiceResolver {
    /// Create a new service resolver
    pub fn new(service_type: ServiceType) -> Self {
        ServiceResolver {
            services: Arc::new(RwLock::new(HashMap::new())),
            service_type,
        }
    }

    /// Start service discovery
    pub async fn start_discovery(&self) -> KvmResult<()> {
        // TODO: Implement mDNS discovery
        println!("Starting service discovery for {:?}", self.service_type);
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

    /// Register a service
    pub async fn register_service(&self, info: ServiceInfo) -> KvmResult<()> {
        let mut services = self.services.write().await;
        services.insert(info.id.clone(), info);
        Ok(())
    }

    /// Unregister a service
    pub async fn unregister_service(&self, id: &ServiceId) -> KvmResult<()> {
        let mut services = self.services.write().await;
        services.remove(id);
        Ok(())
    }
}
