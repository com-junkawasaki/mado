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

//! # Soft KVM Server
//!
//! KVM共有サーバーの実装

pub mod config;
pub mod handler;
pub mod manager;

use soft_kvm_core::*;
use soft_kvm_protocol::*;
use soft_kvm_platform::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Server result type
pub type ServerResult<T> = Result<T, ServerError>;

/// Server errors
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Generic server error: {0}")]
    Generic(String),
}

/// KVM Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server_name: String,
    pub bind_address: NetworkAddress,
    pub max_clients: usize,
    pub session_timeout: u64,
    pub heartbeat_interval: u64,
    pub enable_discovery: bool,
    pub enable_security: bool,
    pub video_config: VideoConfig,
    pub input_config: InputConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            server_name: "Soft KVM Server".to_string(),
            bind_address: NetworkAddress::localhost(8080),
            max_clients: 5,
            session_timeout: 300, // 5 minutes
            heartbeat_interval: 30,
            enable_discovery: true,
            enable_security: true,
            video_config: default_video_config(),
            input_config: default_input_config(),
        }
    }
}

/// KVM Server
pub struct KvmServer {
    config: ServerConfig,
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    platform_manager: Option<PlatformManager>,
    is_running: Arc<RwLock<bool>>,
}

impl KvmServer {
    /// Create a new KVM server
    pub async fn new(config: ServerConfig) -> ServerResult<Self> {
        info!("Initializing KVM server: {}", config.server_name);

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

        Ok(KvmServer {
            config,
            protocol_manager,
            platform_manager,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the server
    pub async fn start(&mut self) -> ServerResult<()> {
        info!("Starting KVM server on {}", self.config.bind_address);

        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(ServerError::Generic("Server already running".to_string()));
        }
        *is_running = true;
        drop(is_running);

        // Start platform input capture if available
        if let Some(platform) = &mut self.platform_manager {
            platform.start_input_capture(self.config.input_config.clone()).await?;
        }

        // Start protocol server
        let addr = format!("{}:{}", self.config.bind_address.ip, self.config.bind_address.port)
            .parse()
            .map_err(|e| ServerError::Config(format!("Invalid bind address: {}", e)))?;

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.create_server(addr).await?;
        drop(protocol_manager);

        info!("KVM server started successfully");
        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> ServerResult<()> {
        info!("Stopping KVM server");

        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }
        *is_running = false;
        drop(is_running);

        // Stop platform capture
        if let Some(platform) = &mut self.platform_manager {
            platform.stop_input_capture().await?;
        }

        // Stop protocol server
        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.stop_server().await?;
        protocol_manager.stop().await?;
        drop(protocol_manager);

        info!("KVM server stopped successfully");
        Ok(())
    }

    /// Get server status
    pub async fn status(&self) -> ServerResult<ServerStatus> {
        let is_running = *self.is_running.read().await;
        let protocol_manager = self.protocol_manager.read().await;
        let active_sessions = protocol_manager.active_sessions().await;

        Ok(ServerStatus {
            is_running,
            server_name: self.config.server_name.clone(),
            bind_address: self.config.bind_address.clone(),
            active_sessions,
            max_clients: self.config.max_clients,
            platform_supported: self.platform_manager.is_some(),
        })
    }

    /// Get server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

/// Server status information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerStatus {
    pub is_running: bool,
    pub server_name: String,
    pub bind_address: NetworkAddress,
    pub active_sessions: usize,
    pub max_clients: usize,
    pub platform_supported: bool,
}
