//! mDNSリゾルバ実装（簡易版）

use crate::{ServiceInfo, ServiceType, MdnsServiceName};
use soft_kvm_core::{NetworkAddress, KvmResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use tracing::{debug, info};

/// サービスディスカバリリゾルバ
pub struct ServiceResolver {
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    service_type: ServiceType,
}

impl ServiceResolver {
    pub fn new(service_type: ServiceType) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            service_type,
        }
    }

    /// サービスディスカバリを開始
    pub async fn start_discovery(&self) -> KvmResult<()> {
        let service_name = MdnsServiceName::from_service_type(self.service_type);

        info!("Starting mDNS discovery for {} (simplified implementation)", service_name);

        // 簡易実装: 定期的なチェックのみ
        let services = self.services.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(30));

            loop {
                ticker.tick().await;
                debug!("Service discovery tick");
            }
        });

        Ok(())
    }

    /// メタデータからサービス情報をパース（簡易実装）
    fn parse_service_info(_metadata: HashMap<String, String>) -> KvmResult<ServiceInfo> {
        // TODO: 実際のmDNS実装時に詳細を実装
        Err(soft_kvm_core::KvmError::Protocol("mDNS not implemented yet".to_string()))
    }

    /// 利用可能なサービスを取得
    pub async fn get_available_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services
            .values()
            .filter(|service| !service.is_expired())
            .cloned()
            .collect()
    }

    /// サービスを登録
    pub async fn register_service(&self, info: ServiceInfo) -> KvmResult<()> {
        let mut services = self.services.write().await;
        services.insert(info.name.clone(), info);
        Ok(())
    }

    /// 期限切れサービスをクリーンアップ
    pub async fn cleanup_expired_services(&self) {
        let mut services = self.services.write().await;
        let initial_count = services.len();

        services.retain(|_, service| !service.is_expired());

        let removed_count = initial_count - services.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired services", removed_count);
        }
    }
}