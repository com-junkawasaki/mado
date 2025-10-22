//! 証明書管理

use soft_kvm_core::KvmResult;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, Certificate};
use rustls::{Certificate as RustlsCertificate, PrivateKey as RustlsPrivateKey};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use chrono::{DateTime, Utc, Duration};

/// 証明書タイプ
#[derive(Debug, Clone, Copy)]
pub enum CertificateType {
    Server,
    Client,
}

/// 証明書設定
#[derive(Debug, Clone)]
pub struct CertificateConfig {
    pub common_name: String,
    pub organization: String,
    pub organizational_unit: Option<String>,
    pub country: String,
    pub state: String,
    pub locality: String,
    pub validity_days: i64,
    pub key_size: usize,
}

impl Default for CertificateConfig {
    fn default() -> Self {
        Self {
            common_name: "soft-kvm.local".to_string(),
            organization: "Soft KVM Team".to_string(),
            organizational_unit: Some("Development".to_string()),
            country: "JP".to_string(),
            state: "Tokyo".to_string(),
            locality: "Tokyo".to_string(),
            validity_days: 365,
            key_size: 2048,
        }
    }
}

/// 証明書マネージャー
pub struct CertificateManager {
    config: CertificateConfig,
    certificates: HashMap<String, (RustlsCertificate, RustlsPrivateKey)>,
}

impl CertificateManager {
    pub fn new(config: CertificateConfig) -> Self {
        Self {
            config,
            certificates: HashMap::new(),
        }
    }

    /// 自己署名証明書を生成
    pub fn generate_self_signed(&mut self, cert_type: CertificateType) -> KvmResult<&(RustlsCertificate, RustlsPrivateKey)> {
        let key_name = match cert_type {
            CertificateType::Server => "server",
            CertificateType::Client => "client",
        };

        if let Some(cert) = self.certificates.get(key_name) {
            return Ok(cert);
        }

        info!("Generating self-signed {} certificate", key_name);

        let key_pair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)?;
        let mut params = CertificateParams::default();

        // 識別名設定
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, &self.config.common_name);
        dn.push(DnType::OrganizationName, &self.config.organization);
        if let Some(ou) = &self.config.organizational_unit {
            dn.push(DnType::OrganizationalUnitName, ou);
        }
        dn.push(DnType::CountryName, &self.config.country);
        dn.push(DnType::StateOrProvinceName, &self.config.state);
        dn.push(DnType::LocalityName, &self.config.locality);
        params.distinguished_name = dn;

        // 有効期間設定
        let now = Utc::now();
        params.not_before = now;
        params.not_after = now + Duration::days(self.config.validity_days);

        // キー使用設定
        match cert_type {
            CertificateType::Server => {
                params.use_authority_key_identifier_extension = true;
                params.key_usages = vec![
                    rcgen::KeyUsagePurpose::DigitalSignature,
                    rcgen::KeyUsagePurpose::KeyEncipherment,
                ];
                params.extended_key_usages = vec![
                    rcgen::ExtendedKeyUsagePurpose::ServerAuth,
                ];
            }
            CertificateType::Client => {
                params.use_authority_key_identifier_extension = true;
                params.key_usages = vec![
                    rcgen::KeyUsagePurpose::DigitalSignature,
                    rcgen::KeyUsagePurpose::KeyEncipherment,
                ];
                params.extended_key_usages = vec![
                    rcgen::ExtendedKeyUsagePurpose::ClientAuth,
                ];
            }
        }

        // 証明書生成
        let cert = params.self_sign(&key_pair)?;

        // Rustls形式に変換
        let rustls_cert = RustlsCertificate(cert.pem().as_bytes().to_vec());
        let rustls_key = RustlsPrivateKey(key_pair.serialize_pem().into_bytes());

        let cert_pair = (rustls_cert, rustls_key);
        self.certificates.insert(key_name.to_string(), cert_pair);

        Ok(self.certificates.get(key_name).unwrap())
    }

    /// サーバー証明書を取得
    pub fn get_server_certificate(&mut self) -> KvmResult<&(RustlsCertificate, RustlsPrivateKey)> {
        self.generate_self_signed(CertificateType::Server)
    }

    /// クライアント証明書を取得
    pub fn get_client_certificate(&mut self) -> KvmResult<&(RustlsCertificate, RustlsPrivateKey)> {
        self.generate_self_signed(CertificateType::Client)
    }

    /// 証明書チェーンを取得
    pub fn get_certificate_chain(&mut self) -> KvmResult<Vec<RustlsCertificate>> {
        let (cert, _) = self.get_server_certificate()?;
        Ok(vec![cert.clone()])
    }

    /// 秘密鍵を取得
    pub fn get_private_key(&mut self) -> KvmResult<RustlsPrivateKey> {
        let (_, key) = self.get_server_certificate()?;
        Ok(key.clone())
    }

    /// 証明書をPEM形式で保存
    pub fn save_certificate_pem(&mut self, path: &std::path::Path) -> KvmResult<()> {
        let (cert, _) = self.get_server_certificate()?;
        std::fs::write(path, &cert.0)?;
        info!("Certificate saved to {:?}", path);
        Ok(())
    }

    /// 秘密鍵をPEM形式で保存
    pub fn save_private_key_pem(&mut self, path: &std::path::Path) -> KvmResult<()> {
        let (_, key) = self.get_server_certificate()?;
        std::fs::write(path, &key.0)?;
        info!("Private key saved to {:?}", path);
        Ok(())
    }

    /// 証明書をDER形式で保存
    pub fn save_certificate_der(&mut self, path: &std::path::Path) -> KvmResult<()> {
        let (cert, _) = self.get_server_certificate()?;
        std::fs::write(path, &cert.0)?;
        info!("Certificate saved to {:?}", path);
        Ok(())
    }

    /// 証明書情報を取得
    pub fn get_certificate_info(&mut self) -> KvmResult<CertificateInfo> {
        use x509_parser::prelude::*;

        let (cert, _) = self.get_server_certificate()?;
        let (_, cert_parsed) = X509Certificate::from_der(&cert.0)
            .map_err(|e| soft_kvm_core::KvmError::Security(format!("Failed to parse certificate: {}", e)))?;

        let subject = cert_parsed.subject();
        let issuer = cert_parsed.issuer();
        let validity = cert_parsed.validity();

        Ok(CertificateInfo {
            subject: subject.to_string(),
            issuer: issuer.to_string(),
            not_before: DateTime::from_utc(validity.not_before.to_datetime(), Utc),
            not_after: DateTime::from_utc(validity.not_after.to_datetime(), Utc),
            serial_number: cert_parsed.serial.to_string(),
            fingerprint_sha256: self.get_certificate_fingerprint_sha256()?,
        })
    }

    /// SHA256フィンガープリントを取得
    pub fn get_certificate_fingerprint_sha256(&mut self) -> KvmResult<String> {
        use ring::digest::{Context, SHA256};

        let (cert, _) = self.get_server_certificate()?;
        let mut context = Context::new(&SHA256);
        context.update(&cert.0);
        let digest = context.finish();

        Ok(hex::encode(digest.as_ref()))
    }
}

/// 証明書情報
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub serial_number: String,
    pub fingerprint_sha256: String,
}

/// LAN証明書ストア
pub struct LanCertificateStore {
    certificates: HashMap<String, RustlsCertificate>,
}

impl LanCertificateStore {
    pub fn new() -> Self {
        Self {
            certificates: HashMap::new(),
        }
    }

    /// 証明書を追加
    pub fn add_certificate(&mut self, hostname: &str, certificate: RustlsCertificate) {
        self.certificates.insert(hostname.to_string(), certificate);
    }

    /// 証明書を取得
    pub fn get_certificate(&self, hostname: &str) -> Option<&RustlsCertificate> {
        self.certificates.get(hostname)
    }

    /// RootCertStoreに変換
    pub fn to_root_store(&self) -> rustls::RootCertStore {
        let mut store = rustls::RootCertStore::empty();

        for cert in self.certificates.values() {
            if let Err(e) = store.add(cert) {
                warn!("Failed to add certificate to store: {}", e);
            }
        }

        store
    }

    /// デフォルトLAN証明書を追加（自己署名）
    pub fn add_default_lan_certificates(&mut self) -> KvmResult<()> {
        let mut cert_manager = CertificateManager::new(CertificateConfig::default());
        let (cert, _) = cert_manager.get_server_certificate()?;

        // 一般的なLANホスト名
        let lan_hostnames = vec![
            "soft-kvm-server.local",
            "soft-kvm.local",
            "localhost",
        ];

        for hostname in lan_hostnames {
            self.add_certificate(hostname, cert.clone());
        }

        debug!("Added default LAN certificates for {} hostnames", lan_hostnames.len());
        Ok(())
    }
}

impl Default for LanCertificateStore {
    fn default() -> Self {
        let mut store = Self::new();
        if let Err(e) = store.add_default_lan_certificates() {
            warn!("Failed to add default LAN certificates: {}", e);
        }
        store
    }
}
