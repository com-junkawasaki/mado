//! TLS 1.3 実装

use soft_kvm_core::KvmResult;
use rustls::{ClientConfig, ServerConfig, RootCertStore, Certificate, PrivateKey};
use tokio_rustls::{TlsConnector, TlsAcceptor, client::TlsStream as ClientTlsStream, server::TlsStream as ServerTlsStream};
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

/// TLS接続タイプ
#[derive(Debug, Clone, Copy)]
pub enum TlsConnectionType {
    Server,
    Client,
}

/// TLS設定
pub struct TlsConfig {
    pub connection_type: TlsConnectionType,
    pub cert_chain: Vec<Certificate>,
    pub private_key: Option<PrivateKey>,
    pub ca_certificates: RootCertStore,
    pub alpn_protocols: Vec<Vec<u8>>,
}

impl TlsConfig {
    /// サーバー設定を作成
    pub fn server(cert_chain: Vec<Certificate>, private_key: PrivateKey) -> KvmResult<Self> {
        let mut ca_store = RootCertStore::empty();
        // LAN専用なので自己署名証明書を使用可能にする
        ca_store.add(&cert_chain[0])
            .map_err(|e| soft_kvm_core::KvmError::Security(format!("Failed to add CA certificate: {}", e)))?;

        Ok(Self {
            connection_type: TlsConnectionType::Server,
            cert_chain,
            private_key: Some(private_key),
            ca_certificates: ca_store,
            alpn_protocols: vec![b"soft-kvm/1".to_vec()],
        })
    }

    /// クライアント設定を作成
    pub fn client(ca_certificates: RootCertStore) -> Self {
        Self {
            connection_type: TlsConnectionType::Client,
            cert_chain: Vec::new(),
            private_key: None,
            ca_certificates,
            alpn_protocols: vec![b"soft-kvm/1".to_vec()],
        }
    }

    /// TLS設定をビルド
    pub fn build(&self) -> KvmResult<TlsConnection> {
        match self.connection_type {
            TlsConnectionType::Server => {
                let private_key = self.private_key.as_ref()
                    .ok_or_else(|| soft_kvm_core::KvmError::Security("Private key required for server".to_string()))?;

                let mut config = ServerConfig::builder()
                    .with_safe_defaults()
                    .with_no_client_auth()
                    .with_single_cert(self.cert_chain.clone(), private_key.clone())
                    .map_err(|e| soft_kvm_core::KvmError::Security(format!("Failed to create server config: {}", e)))?;

                config.alpn_protocols = self.alpn_protocols.clone();

                let acceptor = TlsAcceptor::from(Arc::new(config));
                Ok(TlsConnection::Server(acceptor))
            }
            TlsConnectionType::Client => {
                let mut config = ClientConfig::builder()
                    .with_safe_defaults()
                    .with_root_certificates(self.ca_certificates.clone())
                    .with_no_client_auth();

                config.alpn_protocols = self.alpn_protocols.clone();

                let connector = TlsConnector::from(Arc::new(config));
                Ok(TlsConnection::Client(connector))
            }
        }
    }
}

/// TLS接続
pub enum TlsConnection {
    Server(TlsAcceptor),
    Client(TlsConnector),
}

impl TlsConnection {
    /// サーバーとしてTLS接続を受け入れる
    pub async fn accept(&self, stream: TcpStream) -> KvmResult<SecureStream> {
        match self {
            TlsConnection::Server(acceptor) => {
                debug!("Accepting TLS connection from client");
                let tls_stream = acceptor.accept(stream).await?;
                info!("TLS handshake completed successfully");
                Ok(SecureStream::Server(tls_stream))
            }
            TlsConnection::Client(_) => {
                Err(soft_kvm_core::KvmError::Security("Cannot accept on client connection".to_string()))
            }
        }
    }

    /// クライアントとしてTLS接続を確立
    pub async fn connect(&self, stream: TcpStream, server_name: &str) -> KvmResult<SecureStream> {
        match self {
            TlsConnection::Client(connector) => {
                debug!("Connecting to TLS server: {}", server_name);
                let server_name = rustls::ServerName::try_from(server_name)
                    .map_err(|e| soft_kvm_core::KvmError::Security(format!("Invalid server name: {}", e)))?;

                let tls_stream = connector.connect(server_name.clone(), stream).await?;
                info!("TLS connection established to {:?}", server_name);
                Ok(SecureStream::Client(tls_stream))
            }
            TlsConnection::Server(_) => {
                Err(soft_kvm_core::KvmError::Security("Cannot connect on server connection".to_string()))
            }
        }
    }
}

/// セキュリティ保護されたストリーム
#[derive(Debug)]
pub enum SecureStream {
    Server(ServerTlsStream<TcpStream>),
    Client(ClientTlsStream<TcpStream>),
}

impl SecureStream {
    /// ストリームを取得（読み取り用）
    pub fn get_ref(&self) -> &TcpStream {
        match self {
            SecureStream::Server(s) => s.get_ref().0,
            SecureStream::Client(c) => c.get_ref().0,
        }
    }

    /// ストリームを取得（書き込み用）
    pub fn get_mut(&mut self) -> &mut TcpStream {
        match self {
            SecureStream::Server(s) => s.get_mut().0,
            SecureStream::Client(c) => c.get_mut().0,
        }
    }

    /// ピア証明書を取得
    pub fn peer_certificates(&self) -> Option<&[Certificate]> {
        match self {
            SecureStream::Server(s) => s.get_ref().1.peer_certificates(),
            SecureStream::Client(c) => c.get_ref().1.peer_certificates(),
        }
    }

    /// プロトコルバージョンを取得
    pub fn protocol_version(&self) -> Option<rustls::ProtocolVersion> {
        match self {
            SecureStream::Server(s) => s.get_ref().1.protocol_version(),
            SecureStream::Client(c) => c.get_ref().1.protocol_version(),
        }
    }

    /// 暗号スイートを取得
    pub fn cipher_suite(&self) -> Option<rustls::SupportedCipherSuite> {
        match self {
            SecureStream::Server(s) => s.get_ref().1.negotiated_cipher_suite(),
            SecureStream::Client(c) => c.get_ref().1.negotiated_cipher_suite(),
        }
    }
}

// tokio::io::AsyncRead/AsyncWrite の実装
impl tokio::io::AsyncRead for SecureStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            SecureStream::Server(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            SecureStream::Client(c) => std::pin::Pin::new(c).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for SecureStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            SecureStream::Server(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            SecureStream::Client(c) => std::pin::Pin::new(c).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            SecureStream::Server(s) => std::pin::Pin::new(s).poll_flush(cx),
            SecureStream::Client(c) => std::pin::Pin::new(c).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            SecureStream::Server(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            SecureStream::Client(c) => std::pin::Pin::new(c).poll_shutdown(cx),
        }
    }
}

/// TLS統計情報
#[derive(Debug, Clone)]
pub struct TlsStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub handshake_failures: u64,
    pub protocol_version: Option<String>,
    pub cipher_suite: Option<String>,
}

impl TlsStats {
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            handshake_failures: 0,
            protocol_version: None,
            cipher_suite: None,
        }
    }

    pub fn record_connection(&mut self, stream: &SecureStream) {
        self.total_connections += 1;
        self.active_connections += 1;

        if let Some(version) = stream.protocol_version() {
            self.protocol_version = Some(format!("{:?}", version));
        }

        if let Some(suite) = stream.cipher_suite() {
            self.cipher_suite = Some(format!("{:?}", suite));
        }
    }

    pub fn record_handshake_failure(&mut self) {
        self.handshake_failures += 1;
    }

    pub fn record_connection_closed(&mut self) {
        if self.active_connections > 0 {
            self.active_connections -= 1;
        }
    }
}

/// TLSマネージャー
pub struct TlsManager {
    config: TlsConfig,
    stats: Arc<std::sync::Mutex<TlsStats>>,
}

impl TlsManager {
    pub fn new(config: TlsConfig) -> Self {
        Self {
            config,
            stats: Arc::new(std::sync::Mutex::new(TlsStats::new())),
        }
    }

    pub fn from_config(config: TlsConfig) -> KvmResult<Self> {
        Ok(Self::new(config))
    }

    pub fn get_stats(&self) -> TlsStats {
        self.stats.lock().unwrap().clone()
    }

    pub async fn create_connection(&self) -> KvmResult<TlsConnection> {
        self.config.build()
    }

    /// 証明書フィンガープリントを取得
    pub fn get_certificate_fingerprint(&self) -> Option<String> {
        use ring::digest::{Context, SHA256};

        if self.config.cert_chain.is_empty() {
            return None;
        }

        let cert_der = self.config.cert_chain[0].0.clone();
        let mut context = Context::new(&SHA256);
        context.update(&cert_der);
        let digest = context.finish();

        Some(hex::encode(digest.as_ref()))
    }
}
