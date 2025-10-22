//! mDNSリゾルバ実装

use crate::{ServiceInfo, ServiceType, MdnsServiceName, TxtRecordParser};
use soft_kvm_core::{NetworkAddress, KvmResult, Resolution, VideoQuality};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info, warn};

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

        info!("Starting mDNS discovery for {}", service_name);

        // mDNSクライアントの初期化
        let client = mdns::Client::new(service_name)?;

        // 定期的なサービス検索
        let services = self.services.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                if let Err(e) = Self::discover_services(&client, &services).await {
                    error!("Service discovery error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// サービスを検索
    async fn discover_services(
        client: &mdns::Client,
        services: &RwLock<HashMap<String, ServiceInfo>>,
    ) -> KvmResult<()> {
        let mut stream = client.discover()?;

        while let Some(Ok(response)) = stream.next().await {
            for record in response.records() {
                if let mdns::RecordKind::TXT(txt) = &record.kind {
                    if let Ok(metadata) = TxtRecordParser::parse(txt) {
                        if let Ok(service_info) = Self::parse_service_info(metadata) {
                            let mut services = services.write().await;
                            services.insert(service_info.name.clone(), service_info.clone());
                            debug!("Discovered service: {}", service_info.name);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// メタデータからサービス情報をパース
    fn parse_service_info(metadata: HashMap<String, String>) -> KvmResult<ServiceInfo> {
        let id = metadata
            .get("id")
            .and_then(|id| id.parse().ok())
            .map(soft_kvm_core::ServiceId)
            .unwrap_or_else(soft_kvm_core::ServiceId::new);

        let name = metadata
            .get("name")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let service_type = metadata
            .get("type")
            .and_then(|t| match t.as_str() {
                "Server" => Some(ServiceType::Server),
                "Client" => Some(ServiceType::Client),
                _ => None,
            })
            .unwrap_or(ServiceType::Server);

        let address = Self::parse_address(&metadata)?;

        let version = metadata
            .get("version")
            .cloned()
            .unwrap_or_else(|| "0.1.0".to_string());

        let capabilities = Self::parse_capabilities(&metadata);

        Ok(ServiceInfo::new(service_type, name, address, capabilities))
    }

    /// アドレスをパース
    fn parse_address(metadata: &HashMap<String, String>) -> KvmResult<NetworkAddress> {
        let address_str = metadata
            .get("address")
            .ok_or_else(|| soft_kvm_core::KvmError::Protocol("Missing address".to_string()))?;

        let parts: Vec<&str> = address_str.split(':').collect();
        if parts.len() != 2 {
            return Err(soft_kvm_core::KvmError::Protocol("Invalid address format".to_string()));
        }

        let ip: IpAddr = parts[0].parse()
            .map_err(|_| soft_kvm_core::KvmError::Protocol("Invalid IP address".to_string()))?;
        let port: u16 = parts[1].parse()
            .map_err(|_| soft_kvm_core::KvmError::Protocol("Invalid port".to_string()))?;

        Ok(NetworkAddress::new(ip, port))
    }

    /// 機能をパース
    fn parse_capabilities(metadata: &HashMap<String, String>) -> crate::ServiceCapabilities {
        let mut capabilities = crate::ServiceCapabilities::default();

        // 解像度をパース
        capabilities.supported_resolutions = metadata
            .iter()
            .filter(|(k, _)| k.starts_with("res="))
            .filter_map(|(_, v)| Self::parse_resolution(v))
            .collect();

        // 品質をパース
        capabilities.supported_qualities = metadata
            .iter()
            .filter(|(k, _)| k.starts_with("quality="))
            .filter_map(|(_, v)| Self::parse_quality(v))
            .collect();

        // 最大クライアント数をパース
        if let Some(max_clients) = metadata.get("max_clients") {
            if let Ok(count) = max_clients.parse() {
                capabilities.max_clients = count;
            }
        }

        // 機能をパース
        capabilities.features = metadata
            .iter()
            .filter(|(k, _)| k.starts_with("feature="))
            .map(|(_, v)| v.clone())
            .collect();

        capabilities
    }

    /// 解像度をパース
    fn parse_resolution(res_str: &str) -> Option<Resolution> {
        let parts: Vec<&str> = res_str.split('x').collect();
        if parts.len() == 2 {
            if let (Ok(width), Ok(height)) = (parts[0].parse(), parts[1].parse()) {
                return Some(Resolution::new(width, height));
            }
        }
        None
    }

    /// 品質をパース
    fn parse_quality(quality_str: &str) -> Option<VideoQuality> {
        // fps{fps}Mbps{bitrate} の形式をパース
        if let (Some(fps_start), Some(mbps_start)) = (
            quality_str.find("fps"),
            quality_str.find("Mbps")
        ) {
            let fps_end = mbps_start;
            let mbps_end = quality_str.len();

            if let (Ok(fps), Ok(bitrate)) = (
                quality_str[fps_start + 3..fps_end].parse(),
                quality_str[mbps_start + 4..mbps_end].parse()
            ) {
                return Some(VideoQuality {
                    fps,
                    bitrate_mbps: bitrate,
                    compression_quality: 0.8, // デフォルト値
                });
            }
        }
        None
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
