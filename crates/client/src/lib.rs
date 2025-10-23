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

//! # Soft KVM Client
//!
//! KVM共有クライアントの実装

pub mod config;
pub mod handler;
pub mod manager;

use soft_kvm_core::*;
use soft_kvm_protocol::{messages::*, session::*, *};
use soft_kvm_platform::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Client result type
pub type ClientResult<T> = Result<T, ClientError>;

/// Client errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Generic client error: {0}")]
    Generic(String),
}

/// KVM Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub client_name: String,
    pub server_address: NetworkAddress,
    pub auto_connect: bool,
    pub reconnect_attempts: u32,
    pub reconnect_delay: u64,
    pub session_timeout: u64,
    pub heartbeat_interval: u64,
    pub enable_discovery: bool,
    pub enable_security: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            client_name: format!("Soft KVM Client ({})", whoami::username()),
            server_address: NetworkAddress {
                ip: "127.0.0.1".to_string(),
                port: 8080,
            },
            auto_connect: false,
            reconnect_attempts: 3,
            reconnect_delay: 5,
            session_timeout: 300,
            heartbeat_interval: 30,
            enable_discovery: true,
            enable_security: true,
        }
    }
}

/// KVM Client
pub struct KvmClient {
    config: ClientConfig,
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    platform_manager: Option<PlatformManager>,
    handler: handler::ClientMessageHandler,
    is_connected: Arc<RwLock<bool>>,
    session_id: Arc<RwLock<Option<String>>>,
}

impl KvmClient {
    /// Create a new KVM client
    pub async fn new(config: ClientConfig) -> ClientResult<Self> {
        info!("Initializing KVM client: {}", config.client_name);

        // Initialize platform manager
        let platform_manager = if cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows")) {
            Some(PlatformManager::new()?)
        } else {
            None
        };

        // Create protocol configuration
        let protocol_config = ProtocolConfig {
            version: PROTOCOL_VERSION.to_string(),
            max_message_size: 1024 * 1024, // 1MB
            heartbeat_interval: config.heartbeat_interval,
            session_timeout: config.session_timeout,
            compression_enabled: true,
        };

        // Initialize protocol manager
        let protocol_manager = Arc::new(RwLock::new(ProtocolManager::new(protocol_config)));

        // Create message handler
        let handler = handler::ClientMessageHandler::new(
            config.clone(),
            protocol_manager.clone(),
            platform_manager.clone(),
        );

        Ok(KvmClient {
            config,
            protocol_manager,
            platform_manager,
            handler,
            is_connected: Arc::new(RwLock::new(false)),
            session_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to server
    pub async fn connect(&mut self) -> ClientResult<()> {
        info!("Connecting to KVM server at {}", self.config.server_address);

        {
            let is_connected = self.is_connected.read().await;
            if *is_connected {
                return Err(ClientError::Generic("Client already connected".to_string()));
            }
        }

        let addr = format!("{}:{}", self.config.server_address.ip, self.config.server_address.port)
            .parse()
            .map_err(|e| ClientError::Config(format!("Invalid server address: {}", e)))?;

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.create_client(addr).await?;
        drop(protocol_manager);

        // Send hello message
        self.send_hello().await?;

        let mut is_connected = self.is_connected.write().await;
        *is_connected = true;
        info!("KVM client connected successfully");
        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> ClientResult<()> {
        info!("Disconnecting from KVM server");

        {
            let is_connected = self.is_connected.read().await;
            if !*is_connected {
                return Ok(());
            }
        }

        // Send goodbye message
        {
            let session_id = self.session_id.read().await.clone();
            if let Some(session_id) = session_id {
                self.send_goodbye(session_id, "Client disconnecting".to_string()).await?;
            }
        }

        // Disconnect protocol
        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.disconnect_client().await?;
        protocol_manager.stop().await?;
        drop(protocol_manager);

        let mut is_connected = self.is_connected.write().await;
        *is_connected = false;

        let mut session_id = self.session_id.write().await;
        *session_id = None;

        info!("KVM client disconnected successfully");
        Ok(())
    }

    /// Send hello message to server
    async fn send_hello(&mut self) -> ClientResult<()> {
        let hello_payload = HelloPayload {
            protocol_version: PROTOCOL_VERSION.to_string(),
            client_info: ClientInfo {
                client_id: uuid::Uuid::new_v4().to_string(),
                client_name: self.config.client_name.clone(),
                platform: std::env::consts::OS.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
        };

        let message = ProtocolMessage::new(
            MessageType::Hello,
            MessagePayload::Hello(hello_payload),
        );

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Send goodbye message
    async fn send_goodbye(&mut self, session_id: String, reason: String) -> ClientResult<()> {
        let goodbye_payload = GoodbyePayload {
            reason,
            code: 1000, // Normal closure
        };

        let message = ProtocolMessage::new(
            MessageType::Goodbye,
            MessagePayload::Goodbye(goodbye_payload),
        ).with_session(session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Send keyboard event
    pub async fn send_keyboard_event(&mut self, event: KeyboardEvent) -> ClientResult<()> {
        if !*self.is_connected.read().await {
            return Err(ClientError::Connection("Not connected to server".to_string()));
        }

        let session_id = self.session_id.read().await.clone()
            .ok_or_else(|| ClientError::Connection("No active session".to_string()))?;

        let input_payload = InputEventPayload {
            event_type: "keyboard".to_string(),
            data: serde_json::to_value(event).unwrap(),
        };

        let message = ProtocolMessage::new(
            MessageType::InputEvent,
            MessagePayload::InputEvent(input_payload),
        ).with_session(session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Send mouse event
    pub async fn send_mouse_event(&mut self, event: MouseEvent) -> ClientResult<()> {
        if !*self.is_connected.read().await {
            return Err(ClientError::Connection("Not connected to server".to_string()));
        }

        let session_id = self.session_id.read().await.clone()
            .ok_or_else(|| ClientError::Connection("No active session".to_string()))?;

        let input_payload = InputEventPayload {
            event_type: "mouse".to_string(),
            data: serde_json::to_value(event).unwrap(),
        };

        let message = ProtocolMessage::new(
            MessageType::InputEvent,
            MessagePayload::InputEvent(input_payload),
        ).with_session(session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Get client status
    pub async fn status(&self) -> ClientResult<ClientStatus> {
        let is_connected = *self.is_connected.read().await;
        let session_id = self.session_id.read().await.clone();

        Ok(ClientStatus {
            is_connected,
            client_name: self.config.client_name.clone(),
            server_address: self.config.server_address.clone(),
            session_id,
            platform_supported: self.platform_manager.is_some(),
        })
    }

    /// Get client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Set session ID (called by message handler)
    pub(crate) async fn set_session_id(&self, session_id: String) {
        let mut current_session_id = self.session_id.write().await;
        *current_session_id = Some(session_id);
    }
}

/// Client status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientStatus {
    pub is_connected: bool,
    pub client_name: String,
    pub server_address: NetworkAddress,
    pub session_id: Option<String>,
    pub platform_supported: bool,
}
