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
use tracing::{debug, info, warn, error};

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

/// Protocol manager
pub struct ProtocolManager {
    config: ProtocolConfig,
    sessions: Arc<RwLock<std::collections::HashMap<String, session::ProtocolSession>>>,
}

impl ProtocolManager {
    /// Create a new protocol manager
    pub fn new(config: ProtocolConfig) -> Self {
        ProtocolManager {
            config,
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
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
