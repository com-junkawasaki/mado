//! TLSハンドシェイク処理

use crate::{TlsConnection, SecureStream, CertificateManager, LanCertificateStore};
use soft_kvm_core::{NetworkAddress, KvmResult, ServiceId};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn, error};
use std::sync::Arc;
use std::collections::HashMap;

/// ハンドシェイク設定
#[derive(Debug, Clone)]
pub struct HandshakeConfig {
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub expected_fingerprint: Option<String>,
    pub allow_self_signed: bool,
}

impl Default for HandshakeConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            expected_fingerprint: None,
            allow_self_signed: true, // LAN専用なので自己署名証明書を許可
        }
    }
}

/// ハンドシェイク結果
#[derive(Debug)]
pub struct HandshakeResult {
    pub stream: SecureStream,
    pub peer_info: PeerInfo,
    pub handshake_time_ms: f64,
    pub protocol_version: String,
    pub cipher_suite: String,
}

/// ピア情報
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub service_id: Option<ServiceId>,
    pub address: NetworkAddress,
    pub certificate_fingerprint: Option<String>,
    pub authenticated: bool,
}

/// TLSハンドシェイクマネージャー
pub struct HandshakeManager {
    config: HandshakeConfig,
    cert_manager: CertificateManager,
    cert_store: LanCertificateStore,
    stats: Arc<std::sync::Mutex<HandshakeStats>>,
}

impl HandshakeManager {
    pub fn new(config: HandshakeConfig, cert_config: crate::CertificateConfig) -> Self {
        Self {
            config,
            cert_manager: CertificateManager::new(cert_config),
            cert_store: LanCertificateStore::default(),
            stats: Arc::new(std::sync::Mutex::new(HandshakeStats::new())),
        }
    }

    /// サーバーハンドシェイクを実行
    pub async fn perform_server_handshake(
        &mut self,
        stream: TcpStream,
        expected_client_id: Option<ServiceId>,
    ) -> KvmResult<HandshakeResult> {
        let start_time = std::time::Instant::now();

        debug!("Starting server TLS handshake");

        // TLS接続設定
        let (cert, key) = self.cert_manager.get_server_certificate()?;
        let tls_config = crate::TlsConfig::server(vec![cert.clone()], key.clone())?;
        let tls_connection = tls_config.build()?;

        // ハンドシェイク実行（タイムアウト付き）
        let handshake_future = tls_connection.accept(stream);
        let stream = timeout(Duration::from_secs(self.config.timeout_seconds), handshake_future)
            .await
            .map_err(|_| soft_kvm_core::KvmError::Timeout("TLS handshake timeout".to_string()))??;

        let handshake_time = start_time.elapsed().as_secs_f64() * 1000.0;

        // ピア情報収集
        let peer_info = self.collect_peer_info(&stream, expected_client_id).await?;

        // 認証チェック
        self.verify_peer_authentication(&peer_info)?;

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record_successful_handshake(handshake_time);
        }

        let result = HandshakeResult {
            stream,
            peer_info,
            handshake_time_ms: handshake_time,
            protocol_version: self.get_protocol_version(&stream),
            cipher_suite: self.get_cipher_suite(&stream),
        };

        info!("Server TLS handshake completed in {:.2}ms", handshake_time);
        Ok(result)
    }

    /// クライアントハンドシェイクを実行
    pub async fn perform_client_handshake(
        &mut self,
        stream: TcpStream,
        server_address: &NetworkAddress,
        server_fingerprint: Option<&str>,
    ) -> KvmResult<HandshakeResult> {
        let start_time = std::time::Instant::now();

        debug!("Starting client TLS handshake to {}", server_address.ip);

        // TLS接続設定
        let tls_config = crate::TlsConfig::client(self.cert_store.to_root_store());
        let tls_connection = tls_config.build()?;

        // サーバー名設定（LANホスト名）
        let server_name = format!("soft-kvm-server-{}", server_address.ip);

        // ハンドシェイク実行（タイムアウト付き）
        let handshake_future = tls_connection.connect(stream, &server_name);
        let stream = timeout(Duration::from_secs(self.config.timeout_seconds), handshake_future)
            .await
            .map_err(|_| soft_kvm_core::KvmError::Timeout("TLS handshake timeout".to_string()))??;

        let handshake_time = start_time.elapsed().as_secs_f64() * 1000.0;

        // ピア情報収集
        let peer_info = self.collect_peer_info(&stream, None).await?;

        // サーバー証明書検証
        if let Some(expected_fp) = server_fingerprint {
            if let Some(cert_fp) = &peer_info.certificate_fingerprint {
                if cert_fp != expected_fp {
                    return Err(soft_kvm_core::KvmError::Security(
                        format!("Server certificate fingerprint mismatch: expected {}, got {}",
                               expected_fp, cert_fp)
                    ));
                }
            } else {
                warn!("Server certificate fingerprint not available for verification");
            }
        }

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record_successful_handshake(handshake_time);
        }

        let result = HandshakeResult {
            stream,
            peer_info,
            handshake_time_ms: handshake_time,
            protocol_version: self.get_protocol_version(&stream),
            cipher_suite: self.get_cipher_suite(&stream),
        };

        info!("Client TLS handshake completed in {:.2}ms", handshake_time);
        Ok(result)
    }

    /// ピア情報を収集
    async fn collect_peer_info(&self, stream: &SecureStream, expected_id: Option<ServiceId>) -> KvmResult<PeerInfo> {
        let address = NetworkAddress::new(stream.get_ref().peer_addr()?.ip(), 0);

        let certificate_fingerprint = stream.peer_certificates()
            .and_then(|certs| certs.first())
            .and_then(|cert| {
                use ring::digest::{Context, SHA256};
                let mut context = Context::new(&SHA256);
                context.update(&cert.0);
                let digest = context.finish();
                Some(hex::encode(digest.as_ref()))
            });

        // サービスIDは証明書のSubject Alternative Nameから取得（実装予定）
        let service_id = expected_id; // 簡易実装

        let authenticated = certificate_fingerprint.is_some();

        Ok(PeerInfo {
            service_id,
            address,
            certificate_fingerprint,
            authenticated,
        })
    }

    /// ピア認証を検証
    fn verify_peer_authentication(&self, peer_info: &PeerInfo) -> KvmResult<()> {
        if !peer_info.authenticated {
            return Err(soft_kvm_core::KvmError::Security("Peer not authenticated".to_string()));
        }

        // LAN専用なので証明書フィンガープリントチェックをオプションに
        if let Some(expected_fp) = &self.config.expected_fingerprint {
            if let Some(cert_fp) = &peer_info.certificate_fingerprint {
                if cert_fp != expected_fp {
                    return Err(soft_kvm_core::KvmError::Security(
                        format!("Certificate fingerprint mismatch: expected {}, got {}",
                               expected_fp, cert_fp)
                    ));
                }
            }
        }

        Ok(())
    }

    /// プロトコルバージョンを取得
    fn get_protocol_version(&self, stream: &SecureStream) -> String {
        stream.protocol_version()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// 暗号スイートを取得
    fn get_cipher_suite(&self, stream: &SecureStream) -> String {
        stream.cipher_suite()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> HandshakeStats {
        self.stats.lock().unwrap().clone()
    }

    /// 証明書マネージャーにアクセス
    pub fn cert_manager(&mut self) -> &mut CertificateManager {
        &mut self.cert_manager
    }

    /// 証明書ストアにアクセス
    pub fn cert_store(&mut self) -> &mut LanCertificateStore {
        &mut self.cert_store
    }
}

/// ハンドシェイク統計
#[derive(Debug, Clone)]
pub struct HandshakeStats {
    pub total_handshakes: u64,
    pub successful_handshakes: u64,
    pub failed_handshakes: u64,
    pub average_handshake_time_ms: f64,
    pub min_handshake_time_ms: f64,
    pub max_handshake_time_ms: f64,
    pub p95_handshake_time_ms: f64,
    pub p99_handshake_time_ms: f64,
}

impl HandshakeStats {
    pub fn new() -> Self {
        Self {
            total_handshakes: 0,
            successful_handshakes: 0,
            failed_handshakes: 0,
            average_handshake_time_ms: 0.0,
            min_handshake_time_ms: f64::INFINITY,
            max_handshake_time_ms: 0.0,
            p95_handshake_time_ms: 0.0,
            p99_handshake_time_ms: 0.0,
        }
    }

    pub fn record_successful_handshake(&mut self, handshake_time_ms: f64) {
        self.total_handshakes += 1;
        self.successful_handshakes += 1;

        // 統計更新
        self.average_handshake_time_ms =
            (self.average_handshake_time_ms * (self.successful_handshakes - 1) as f64 + handshake_time_ms)
            / self.successful_handshakes as f64;

        self.min_handshake_time_ms = self.min_handshake_time_ms.min(handshake_time_ms);
        self.max_handshake_time_ms = self.max_handshake_time_ms.max(handshake_time_ms);

        // 簡易パーセンタイル計算
        self.p95_handshake_time_ms = self.max_handshake_time_ms * 0.95;
        self.p99_handshake_time_ms = self.max_handshake_time_ms * 0.99;
    }

    pub fn record_failed_handshake(&mut self) {
        self.total_handshakes += 1;
        self.failed_handshakes += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_handshakes == 0 {
            0.0
        } else {
            self.successful_handshakes as f64 / self.total_handshakes as f64
        }
    }
}

/// セキュア接続ビルダー
pub struct SecureConnectionBuilder {
    handshake_config: HandshakeConfig,
    cert_config: crate::CertificateConfig,
}

impl SecureConnectionBuilder {
    pub fn new() -> Self {
        Self {
            handshake_config: HandshakeConfig::default(),
            cert_config: crate::CertificateConfig::default(),
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.handshake_config.timeout_seconds = seconds;
        self
    }

    pub fn with_expected_fingerprint(mut self, fingerprint: String) -> Self {
        self.handshake_config.expected_fingerprint = Some(fingerprint);
        self
    }

    pub fn with_certificate_config(mut self, config: crate::CertificateConfig) -> Self {
        self.cert_config = config;
        self
    }

    pub fn build(self) -> HandshakeManager {
        HandshakeManager::new(self.handshake_config, self.cert_config)
    }
}

impl Default for SecureConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}
