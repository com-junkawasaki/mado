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

//! # Soft KVM Protocol
//!
//! KVM共有プロトコルの実装

pub mod messages;
pub mod transport;
pub mod websocket;
pub mod session;

use soft_kvm_core::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::net::SocketAddr;
use tracing::{debug, info, warn, error};
use crate::transport::TransportFactory;

/// Protocol result type
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Protocol errors
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Protocol version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },

    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),

    #[error("Generic protocol error: {0}")]
    Generic(String),
}

/// Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    pub version: String,
    pub max_message_size: usize,
    pub heartbeat_interval: u64, // seconds
    pub session_timeout: u64,    // seconds
    pub compression_enabled: bool,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        ProtocolConfig {
            version: PROTOCOL_VERSION.to_string(),
            max_message_size: 1024 * 1024, // 1MB
            heartbeat_interval: 30,
            session_timeout: 300, // 5 minutes
            compression_enabled: true,
        }
    }
}

/// Protocol Server
pub struct ProtocolServer {
    config: ProtocolConfig,
    listener: Option<Arc<tokio::sync::Mutex<Box<dyn transport::TransportListener>>>>,
    sessions: Arc<RwLock<std::collections::HashMap<String, session::ProtocolSession>>>,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
}

impl ProtocolServer {
    /// Create a new protocol server
    pub fn new(config: ProtocolConfig) -> ProtocolResult<Self> {
        let (shutdown_sender, _) = tokio::sync::broadcast::channel(1);

        Ok(ProtocolServer {
            config,
            listener: None,
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            shutdown_sender,
        })
    }

    /// Start the server
    pub async fn start(&mut self, addr: SocketAddr) -> ProtocolResult<()> {
        info!("Starting protocol server on {}", addr);

        // Create transport listener
        let transport_config = transport::TransportConfig::default();
        let factory = websocket::WebSocketFactory::new(transport_config.clone());
        let listener = factory.create_listener(addr, transport_config).await?;

        self.listener = Some(Arc::new(tokio::sync::Mutex::new(listener)));

        // Start accepting connections
        self.start_accept_loop();

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> ProtocolResult<()> {
        info!("Stopping protocol server");

        // Send shutdown signal
        let _ = self.shutdown_sender.send(());

        // Close listener
        if let Some(listener) = self.listener.take() {
            let mut listener = listener.lock().await;
            listener.close().await?;
        }

        // Close all sessions
        let mut sessions = self.sessions.write().await;
        for (session_id, session) in sessions.drain() {
            if let Err(e) = session.close().await {
                warn!("Failed to close session {}: {}", session_id, e);
            }
        }

        Ok(())
    }

    /// Start the connection accept loop
    fn start_accept_loop(&self) {
        let listener = self.listener.as_ref().unwrap().clone();
        let sessions = self.sessions.clone();
        let config = self.config.clone();
        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = async {
                        let mut listener_guard = listener.lock().await;
                        listener_guard.accept().await
                    } => {
                        let connection = result;
                        match connection {
                            Ok(conn) => {
                                let session_id = format!("server-session-{}", uuid::Uuid::new_v4());
                                let remote_addr = conn.remote_addr()
                                    .unwrap_or_else(|| "127.0.0.1:0".parse().unwrap());

                                let peer_info = session::PeerInfo {
                                    peer_id: "client".to_string(),
                                    peer_name: "KVM Client".to_string(),
                                    address: NetworkAddress {
                                        ip: remote_addr.ip().to_string(),
                                        port: remote_addr.port() as u16,
                                    },
                                    capabilities: vec!["video".to_string(), "input".to_string()],
                                    authenticated: false,
                                    last_seen: chrono::Utc::now(),
                                };

                                let session = session::ProtocolSession::new(
                                    session_id.clone(),
                                    peer_info,
                                    config.clone(),
                                );

                                let mut sessions_write = sessions.write().await;
                                sessions_write.insert(session_id.clone(), session);
                                drop(sessions_write);

                                info!("Accepted new connection: {}", session_id);
                            }
                            Err(e) => {
                                error!("Failed to accept connection: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        info!("Server shutdown signal received");
                        break;
                    }
                }
            }
        });
    }
}

/// Protocol Client
pub struct ProtocolClient {
    config: ProtocolConfig,
    session: Option<session::ProtocolSession>,
    transport_connection: Option<Box<dyn transport::TransportConnection>>,
}

impl ProtocolClient {
    /// Create a new protocol client
    pub fn new(config: ProtocolConfig) -> Self {
        ProtocolClient {
            config,
            session: None,
            transport_connection: None,
        }
    }

    /// Connect to a server
    pub async fn connect(&mut self, addr: SocketAddr) -> ProtocolResult<()> {
        info!("Connecting to server at {}", addr);

        // Create transport connection
        let transport_config = transport::TransportConfig::default();
        let connection = websocket::WebSocketConnection::connect(addr, transport_config).await?;

        self.transport_connection = Some(Box::new(connection));

        // Create session
        let peer_info = session::PeerInfo {
            peer_id: "server".to_string(),
            peer_name: "KVM Server".to_string(),
            address: NetworkAddress {
                ip: addr.ip().to_string(),
                port: addr.port() as u16,
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        let session = session::ProtocolSession::new(
            "client-session".to_string(),
            peer_info,
            self.config.clone(),
        );

        self.session = Some(session);

        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> ProtocolResult<()> {
        info!("Disconnecting from server");

        if let Some(mut connection) = self.transport_connection.take() {
            connection.close().await?;
        }

        if let Some(session) = self.session.take() {
            session.close().await?;
        }

        Ok(())
    }

    /// Send a message
    pub async fn send_message(&mut self, message: messages::ProtocolMessage) -> ProtocolResult<()> {
        if let Some(connection) = &mut self.transport_connection {
            connection.send(message).await?;
            Ok(())
        } else {
            Err(ProtocolError::Transport("Not connected".to_string()))
        }
    }

    /// Receive a message
    pub async fn receive_message(&mut self) -> ProtocolResult<Option<messages::ProtocolMessage>> {
        if let Some(connection) = &mut self.transport_connection {
            connection.receive().await
        } else {
            Err(ProtocolError::Transport("Not connected".to_string()))
        }
    }

    /// Get the current session
    pub fn session(&self) -> Option<&session::ProtocolSession> {
        self.session.as_ref()
    }
}

/// Protocol manager
pub struct ProtocolManager {
    config: ProtocolConfig,
    sessions: Arc<RwLock<std::collections::HashMap<String, session::ProtocolSession>>>,
    server: Option<ProtocolServer>,
    client: Option<ProtocolClient>,
}

impl ProtocolManager {
    /// Create a new protocol manager
    pub fn new(config: ProtocolConfig) -> Self {
        ProtocolManager {
            config,
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            server: None,
            client: None,
        }
    }

    /// Create a default protocol manager
    pub fn default() -> Self {
        Self::new(ProtocolConfig::default())
    }

    /// Start the protocol manager
    pub async fn start(&self) -> ProtocolResult<()> {
        info!("Starting protocol manager with version {}", self.config.version);
        Ok(())
    }

    /// Stop the protocol manager
    pub async fn stop(&self) -> ProtocolResult<()> {
        info!("Stopping protocol manager");

        let mut sessions = self.sessions.write().await;
        for (session_id, session) in sessions.drain() {
            debug!("Terminating session: {}", session_id);
            if let Err(e) = session.close().await {
                warn!("Failed to close session {}: {}", session_id, e);
            }
        }

        Ok(())
    }

    /// Get protocol configuration
    pub fn config(&self) -> &ProtocolConfig {
        &self.config
    }

    /// Create a new session
    pub async fn create_session(&self, session_id: String, peer_info: session::PeerInfo) -> ProtocolResult<()> {
        let session = session::ProtocolSession::new(session_id.clone(), peer_info, self.config.clone());

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        Ok(())
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<session::ProtocolSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> ProtocolResult<()> {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(session_id).is_some() {
            debug!("Removed session: {}", session_id);
        }
        Ok(())
    }

    /// Get active session count
    pub async fn active_sessions(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Send a message to a session
    pub async fn send_message(&self, session_id: &str, message: messages::ProtocolMessage) -> ProtocolResult<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            session.send_message(message).await
        } else {
            Err(ProtocolError::Session(format!("Session not found: {}", session_id)))
        }
    }

    /// Broadcast a message to all sessions
    pub async fn broadcast_message(&self, message: messages::ProtocolMessage) -> ProtocolResult<()> {
        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            if let Err(e) = session.send_message(message.clone()).await {
                warn!("Failed to send message to session: {}", e);
            }
        }
        Ok(())
    }

    /// Create and start a server
    pub async fn create_server(&mut self, addr: SocketAddr) -> ProtocolResult<()> {
        let mut server = ProtocolServer::new(self.config.clone())?;
        server.start(addr).await?;
        self.server = Some(server);
        Ok(())
    }

    /// Stop the server
    pub async fn stop_server(&mut self) -> ProtocolResult<()> {
        if let Some(mut server) = self.server.take() {
            server.stop().await?;
        }
        Ok(())
    }

    /// Create and connect a client
    pub async fn create_client(&mut self, addr: SocketAddr) -> ProtocolResult<()> {
        let mut client = ProtocolClient::new(self.config.clone());
        client.connect(addr).await?;
        self.client = Some(client);
        Ok(())
    }

    /// Disconnect the client
    pub async fn disconnect_client(&mut self) -> ProtocolResult<()> {
        if let Some(mut client) = self.client.take() {
            client.disconnect().await?;
        }
        Ok(())
    }

    /// Send message as client
    pub async fn send_client_message(&mut self, message: messages::ProtocolMessage) -> ProtocolResult<()> {
        if let Some(client) = &mut self.client {
            client.send_message(message).await
        } else {
            Err(ProtocolError::Transport("Client not connected".to_string()))
        }
    }

    /// Receive message as client
    pub async fn receive_client_message(&mut self) -> ProtocolResult<Option<messages::ProtocolMessage>> {
        if let Some(client) = &mut self.client {
            client.receive_message().await
        } else {
            Err(ProtocolError::Transport("Client not connected".to_string()))
        }
    }

    /// Get the current client session
    pub fn client_session(&self) -> Option<&session::ProtocolSession> {
        self.client.as_ref().and_then(|c| c.session())
    }
}

/// Simple test function to verify protocol compilation
pub async fn test_protocol_basic() -> ProtocolResult<()> {
    info!("Testing basic protocol functionality");

    // Create a server
    let config = ProtocolConfig::default();
    let mut server = ProtocolServer::new(config.clone())?;

    // Start server on a test port
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap(); // Use port 0 for auto-assignment
    server.start(addr).await?;

    // Get the actual port the server is listening on
    let server_addr = if let Some(listener) = &server.listener {
        let listener_guard = listener.lock().await;
        listener_guard.local_addr().unwrap_or_else(|| "127.0.0.1:8080".parse().unwrap())
    } else {
        "127.0.0.1:8080".parse().unwrap()
    };

    info!("Server started on {}", server_addr);

    // Create a client
    let mut client = ProtocolClient::new(config);
    client.connect(server_addr).await?;

    info!("Client connected successfully");

    // Send a test message
    let test_message = messages::ProtocolMessage::new(
        messages::MessageType::Hello,
        messages::MessagePayload::Hello(messages::HelloPayload {
            protocol_version: "1.0.0".to_string(),
            client_info: messages::ClientInfo {
                client_id: "test-client".to_string(),
                client_name: "Test KVM Client".to_string(),
                platform: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
        }),
    );

    client.send_message(test_message).await?;
    info!("Test message sent");

    // Disconnect client
    client.disconnect().await?;
    info!("Client disconnected");

    // Stop server
    server.stop().await?;
    info!("Server stopped");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_manager_creation() {
        let manager = ProtocolManager::default();
        assert_eq!(manager.config().version, PROTOCOL_VERSION);
        assert_eq!(manager.active_sessions().await, 0);
    }

    #[tokio::test]
    async fn test_protocol_config_default() {
        let config = ProtocolConfig::default();
        assert_eq!(config.version, PROTOCOL_VERSION);
        assert!(config.compression_enabled);
        assert_eq!(config.heartbeat_interval, 30);
    }
}
