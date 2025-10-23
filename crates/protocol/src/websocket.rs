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

//! WebSocket over TLS transport implementation

use crate::{messages::ProtocolMessage, transport::{TransportConnection, TransportListener, TransportFactory, TransportConfig}, ProtocolResult, ProtocolError};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, connect_async, connect_async_with_config, tungstenite::Message, MaybeTlsStream, Connector};
use tokio_rustls::TlsAcceptor;
use rustls::OwnedTrustAnchor;

/// Dangerous TLS configuration for development
mod dangerous {
    use rustls::client::ServerCertVerifier;
    use rustls::{Certificate, ServerName, Error};

    pub struct NoCertificateVerification;

    impl ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
}
use tracing::{debug, info, warn, error};

/// WebSocket over TLS connection
pub struct WebSocketConnection {
    stream: tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>,
    remote_addr: SocketAddr,
    config: TransportConfig,
    is_alive: bool,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection from a raw stream
    pub async fn new(stream: TcpStream, config: TransportConfig) -> ProtocolResult<Self> {
        let remote_addr = stream.peer_addr()
            .map_err(|e| ProtocolError::Transport(format!("Failed to get peer address: {}", e)))?;

        // Accept WebSocket connection
        let ws_stream = accept_async(tokio_tungstenite::MaybeTlsStream::Plain(stream))
            .await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to accept WebSocket: {}", e)))?;

        Ok(WebSocketConnection {
            stream: ws_stream,
            remote_addr,
            config,
            is_alive: true,
        })
    }

    /// Create a client WebSocket connection with optional TLS
    pub async fn connect(addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Self> {
        let scheme = if config.tls.enabled { "wss" } else { "ws" };
        let url = format!("{}://{}", scheme, addr);
        let url = url::Url::parse(&url)
            .map_err(|e| ProtocolError::Transport(format!("Invalid URL: {}", e)))?;

        // For now, use the simple connect method
        // TLS configuration will be handled by the URL scheme (wss://)
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to connect WebSocket: {}", e)))?;

        Ok(WebSocketConnection {
            stream: ws_stream,
            remote_addr: addr,
            config,
            is_alive: true,
        })
    }
}

#[async_trait]
impl TransportConnection for WebSocketConnection {
    async fn send(&mut self, message: ProtocolMessage) -> ProtocolResult<()> {
        if !self.is_alive {
            return Err(ProtocolError::Transport("Connection is closed".to_string()));
        }

        // Serialize message using JSON
        let data = serde_json::to_string(&message)
            .map_err(|e| ProtocolError::Serialization(e))?;

        // Check message size
        if data.len() > self.config.buffer_size {
            return Err(ProtocolError::Transport(format!(
                "Message size {} exceeds buffer size {}",
                data.len(),
                self.config.buffer_size
            )));
        }

        // Send as WebSocket binary message
        let ws_message = Message::Binary(data.into_bytes());
        self.stream.send(ws_message).await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to send message: {}", e)))?;

        debug!("Sent message to {}", self.remote_addr);
        Ok(())
    }

    async fn receive(&mut self) -> ProtocolResult<Option<ProtocolMessage>> {
        if !self.is_alive {
            return Ok(None);
        }

        // Set read timeout
        let timeout_duration = std::time::Duration::from_secs(self.config.read_timeout);

        match tokio::time::timeout(timeout_duration, self.stream.next()).await {
            Ok(Some(Ok(message))) => {
                match message {
                    Message::Binary(data) => {
                        // Deserialize message using JSON
                        let protocol_message: ProtocolMessage = serde_json::from_slice(&data)
                            .map_err(|e| ProtocolError::Serialization(e))?;

                        debug!("Received message from {}", self.remote_addr);
                        Ok(Some(protocol_message))
                    }
                    Message::Text(text) => {
                        // Handle text messages if needed (for debugging)
                        warn!("Received text message from {}, ignoring: {}", self.remote_addr, text);
                        Ok(None)
                    }
                    Message::Ping(_) => {
                        // Auto-pong is handled by tungstenite
                        Ok(None)
                    }
                    Message::Pong(_) => {
                        // Pong received
                        Ok(None)
                    }
                    Message::Close(_) => {
                        info!("WebSocket close frame received from {}", self.remote_addr);
                        self.is_alive = false;
                        Ok(None)
                    }
                    Message::Frame(_) => {
                        // Raw frame, ignore for now
                        Ok(None)
                    }
                }
            }
            Ok(Some(Err(e))) => {
                error!("WebSocket error from {}: {}", self.remote_addr, e);
                self.is_alive = false;
                Err(ProtocolError::WebSocket(format!("WebSocket error: {}", e)))
            }
            Ok(None) => {
                // Stream ended
                info!("WebSocket stream ended for {}", self.remote_addr);
                self.is_alive = false;
                Ok(None)
            }
            Err(_) => {
                // Timeout
                Err(ProtocolError::Timeout)
            }
        }
    }

    async fn close(&mut self) -> ProtocolResult<()> {
        if self.is_alive {
            self.stream.close(None).await
                .map_err(|e| ProtocolError::WebSocket(format!("Failed to close WebSocket: {}", e)))?;
            self.is_alive = false;
            info!("Closed WebSocket connection to {}", self.remote_addr);
        }
        Ok(())
    }

    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }

    fn is_alive(&self) -> bool {
        self.is_alive
    }
}

/// WebSocket listener with optional TLS
pub struct WebSocketListener {
    listener: TcpListener,
    config: TransportConfig,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
}

impl WebSocketListener {
    /// Create a new WebSocket listener
    pub async fn new(config: TransportConfig, addr: SocketAddr) -> ProtocolResult<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ProtocolError::Transport(format!("Failed to bind listener: {}", e)))?;

        // TLS設定がある場合はTLSアクセプタを作成
        let tls_acceptor = if config.tls.enabled {
            Some(Self::create_tls_acceptor(&config.tls).await?)
        } else {
            None
        };

        info!("WebSocket listener bound to {} (TLS: {})", addr, config.tls.enabled);

        Ok(WebSocketListener {
            listener,
            config,
            tls_acceptor,
        })
    }

    /// Create TLS acceptor from configuration
    async fn create_tls_acceptor(tls_config: &super::transport::TlsConfig) -> ProtocolResult<tokio_rustls::TlsAcceptor> {
        // 証明書と秘密鍵を読み込み
        let cert_path = tls_config.certificate_path.as_ref()
            .ok_or_else(|| ProtocolError::Transport("Certificate path not specified".to_string()))?;
        let key_path = tls_config.private_key_path.as_ref()
            .ok_or_else(|| ProtocolError::Transport("Private key path not specified".to_string()))?;

        // 証明書を読み込み
        let certs = Self::load_certs(cert_path).await?;
        let key = Self::load_private_key(key_path).await?;

        // TLS設定を作成
        let mut config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| ProtocolError::Transport(format!("TLS config error: {}", e)))?;

        Ok(tokio_rustls::TlsAcceptor::from(Arc::new(config)))
    }

    /// Load certificates from file
    async fn load_certs(path: &str) -> ProtocolResult<Vec<rustls::Certificate>> {
        let cert_data = tokio::fs::read(path)
            .await
            .map_err(|e| ProtocolError::Transport(format!("Failed to read certificate file: {}", e)))?;

        let mut reader = std::io::Cursor::new(cert_data);
        rustls_pemfile::certs(&mut reader)
            .map_err(|_| ProtocolError::Transport("Failed to parse certificate".to_string()))?
            .into_iter()
            .map(|cert| Ok(rustls::Certificate(cert)))
            .collect()
    }

    /// Load private key from file
    async fn load_private_key(path: &str) -> ProtocolResult<rustls::PrivateKey> {
        let key_data = tokio::fs::read(path)
            .await
            .map_err(|e| ProtocolError::Transport(format!("Failed to read private key file: {}", e)))?;

        let mut reader = std::io::Cursor::new(key_data);
        let key = rustls_pemfile::pkcs8_private_keys(&mut reader)
            .map_err(|_| ProtocolError::Transport("Failed to parse PKCS8 private key".to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| ProtocolError::Transport("No PKCS8 private key found".to_string()))?;

        Ok(rustls::PrivateKey(key))
    }
}

#[async_trait]
impl TransportListener for WebSocketListener {
    async fn accept(&mut self) -> ProtocolResult<Box<dyn TransportConnection>> {
        let (stream, addr) = self.listener.accept().await
            .map_err(|e| ProtocolError::Transport(format!("Failed to accept connection: {}", e)))?;

        // TLSが有効な場合はTLSストリームを作成
        let maybe_tls_stream = if let Some(_acceptor) = &self.tls_acceptor {
            // TODO: Implement proper TLS server handshake
            // For now, fall back to plain connection
            warn!("TLS server not yet implemented, falling back to plain connection");
            tokio_tungstenite::MaybeTlsStream::Plain(stream)
        } else {
            tokio_tungstenite::MaybeTlsStream::Plain(stream)
        };

        // WebSocket handshake
        let ws_stream = accept_async(maybe_tls_stream)
            .await
            .map_err(|e| ProtocolError::WebSocket(format!("WebSocket handshake failed: {}", e)))?;

        let connection = WebSocketConnection {
            stream: ws_stream,
            remote_addr: addr,
            config: self.config.clone(),
            is_alive: true,
        };

        info!("Accepted WebSocket connection from {} (TLS: {})",
              connection.remote_addr,
              self.config.tls.enabled);

        Ok(Box::new(connection))
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    async fn close(&mut self) -> ProtocolResult<()> {
        // TcpListener doesn't have a close method, but we can drop it
        info!("WebSocket listener closed");
        Ok(())
    }
}

/// WebSocket factory
#[derive(Clone)]
pub struct WebSocketFactory {
    config: TransportConfig,
}

impl WebSocketFactory {
    pub fn new(config: TransportConfig) -> Self {
        WebSocketFactory { config }
    }
}

#[async_trait]
impl TransportFactory for WebSocketFactory {
    async fn create_listener(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportListener>> {
        let listener = WebSocketListener::new(config, addr).await?;
        Ok(Box::new(listener))
    }

    async fn create_connection(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportConnection>> {
        let connection = WebSocketConnection::connect(addr, config).await?;
        Ok(Box::new(connection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_websocket_factory_creation() {
        let factory = WebSocketFactory::new(TransportConfig::default());
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let config = TransportConfig::default();

        // Test listener creation
        let listener_result = factory.create_listener(addr, config.clone()).await;
        assert!(listener_result.is_ok());

        // Test connection creation (will fail since no server is running)
        let connection_result = factory.create_connection(addr, config).await;
        assert!(connection_result.is_err()); // Expected to fail
    }

    #[tokio::test]
    async fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.read_timeout, 30);
        assert_eq!(config.write_timeout, 30);
        assert_eq!(config.buffer_size, 64 * 1024);
        assert!(config.compression);
    }
}
