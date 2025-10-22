//! サービスディスカバリ基本構造

use crate::KvmResult;
use soft_kvm_core::{ServiceId, NetworkAddress, Resolution, VideoQuality};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// サービスタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Server,
    Client,
}

/// サービス情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: ServiceId,
    pub service_type: ServiceType,
    pub name: String,
    pub address: NetworkAddress,
    pub version: String,
    pub capabilities: ServiceCapabilities,
    pub last_seen: SystemTime,
    pub ttl: Duration,
}

impl ServiceInfo {
    pub fn new(
        service_type: ServiceType,
        name: String,
        address: NetworkAddress,
        capabilities: ServiceCapabilities,
    ) -> Self {
        Self {
            id: ServiceId::new(),
            service_type,
            name,
            address,
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities,
            last_seen: SystemTime::now(),
            ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn is_expired(&self) -> bool {
        self.last_seen
            .elapsed()
            .map(|elapsed| elapsed > self.ttl)
            .unwrap_or(true)
    }

    pub fn refresh(&mut self) {
        self.last_seen = SystemTime::now();
    }
}

/// サービス機能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    pub supported_resolutions: Vec<Resolution>,
    pub supported_qualities: Vec<VideoQuality>,
    pub max_clients: usize,
    pub features: Vec<String>,
}

impl Default for ServiceCapabilities {
    fn default() -> Self {
        Self {
            supported_resolutions: vec![
                Resolution::hd(),
                Resolution::fhd(),
                Resolution::qhd(),
            ],
            supported_qualities: vec![
                VideoQuality::low_latency(),
                VideoQuality::balanced(),
                VideoQuality::high_quality(),
            ],
            max_clients: 1,
            features: vec![
                "tls_1_3".to_string(),
                "h264".to_string(),
                "keyboard".to_string(),
                "mouse".to_string(),
            ],
        }
    }
}

/// mDNSサービス名
pub struct MdnsServiceName;

impl MdnsServiceName {
    pub const KVM_SERVER: &'static str = "_soft-kvm-server._tcp.local.";
    pub const KVM_CLIENT: &'static str = "_soft-kvm-client._tcp.local.";

    pub fn from_service_type(service_type: ServiceType) -> &'static str {
        match service_type {
            ServiceType::Server => Self::KVM_SERVER,
            ServiceType::Client => Self::KVM_CLIENT,
        }
    }
}

/// TXTレコードパーサー
pub struct TxtRecordParser;

impl TxtRecordParser {
    pub fn parse(data: &[u8]) -> KvmResult<HashMap<String, String>> {
        let txt = String::from_utf8(data.to_vec())
            .map_err(|_| soft_kvm_core::KvmError::Protocol("Invalid TXT record encoding".to_string()))?;

        let mut map = HashMap::new();
        for entry in txt.split('\0') {
            if let Some((key, value)) = entry.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(map)
    }

    pub fn serialize(info: &ServiceInfo) -> Vec<u8> {
        let mut entries = Vec::new();

        entries.push(format!("id={}", info.id.0));
        entries.push(format!("name={}", info.name));
        entries.push(format!("version={}", info.version));
        entries.push(format!("type={:?}", info.service_type));
        entries.push(format!("address={}:{}", info.address.ip, info.address.port));

        // Capabilities
        for res in &info.capabilities.supported_resolutions {
            entries.push(format!("res={}x{}", res.width, res.height));
        }

        for quality in &info.capabilities.supported_qualities {
            entries.push(format!("quality=fps{}Mbps{}", quality.fps, quality.bitrate_mbps));
        }

        entries.push(format!("max_clients={}", info.capabilities.max_clients));

        for feature in &info.capabilities.features {
            entries.push(format!("feature={}", feature));
        }

        entries.join("\0").into_bytes()
    }
}
