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

//! Protocol session management

use crate::{messages::*, ProtocolConfig, ProtocolError, ProtocolResult};
use soft_kvm_core::*;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn, error};
use serde::{Serialize, Deserialize};

/// Peer information for session
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub address: NetworkAddress,
    pub capabilities: Vec<String>,
    pub authenticated: bool,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Session state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Authenticating,
    Active,
    Suspended,
    Closing,
    Closed,
}

/// Protocol session
#[derive(Debug, Clone)]
pub struct ProtocolSession {
    session_id: String,
    peer_info: PeerInfo,
    config: ProtocolConfig,
    state: Arc<RwLock<SessionState>>,
    message_sender: mpsc::UnboundedSender<ProtocolMessage>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ProtocolMessage>>>>,
    last_activity: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
    heartbeat_sequence: Arc<RwLock<u64>>,
}

impl ProtocolSession {
    /// Create a new protocol session
    pub fn new(session_id: String, peer_info: PeerInfo, config: ProtocolConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        ProtocolSession {
            session_id,
            peer_info,
            config,
            state: Arc::new(RwLock::new(SessionState::Connecting)),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            last_activity: Arc::new(RwLock::new(chrono::Utc::now())),
            heartbeat_sequence: Arc::new(RwLock::new(0)),
        }
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get peer information
    pub fn peer_info(&self) -> &PeerInfo {
        &self.peer_info
    }

    /// Get current session state
    pub async fn state(&self) -> SessionState {
        self.state.read().await.clone()
    }

    /// Set session state
    pub async fn set_state(&self, state: SessionState) {
        let mut current_state = self.state.write().await;
        *current_state = state.clone();
        debug!("Session {} state changed to {:?}", self.session_id, state);
    }

    /// Check if session is active
    pub async fn is_active(&self) -> bool {
        matches!(*self.state.read().await, SessionState::Active)
    }

    /// Check if session is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.peer_info.authenticated
    }

    /// Set authentication status
    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.peer_info.authenticated = authenticated;
    }

    /// Update last activity timestamp
    pub async fn update_activity(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = chrono::Utc::now();
    }

    /// Get last activity timestamp
    pub async fn last_activity(&self) -> chrono::DateTime<chrono::Utc> {
        *self.last_activity.read().await
    }

    /// Check if session has timed out
    pub async fn is_timed_out(&self) -> bool {
        let last_activity = self.last_activity().await;
        let elapsed = chrono::Utc::now().signed_duration_since(last_activity);
        elapsed > chrono::Duration::seconds(self.config.session_timeout as i64)
    }

    /// Send a message through this session
    pub async fn send_message(&self, message: ProtocolMessage) -> ProtocolResult<()> {
        if !self.is_authenticated() && !matches!(message.message_type(), messages::MessageType::Hello | messages::MessageType::AuthRequest) {
            return Err(ProtocolError::Authentication("Session not authenticated".to_string()));
        }

        self.message_sender.send(message)
            .map_err(|e| ProtocolError::Transport(format!("Failed to send message: {}", e)))?;

        self.update_activity().await;
        Ok(())
    }

    /// Try to receive a message from this session
    pub async fn try_receive_message(&self) -> Option<ProtocolMessage> {
        let mut receiver_guard = self.message_receiver.write().await;
        if let Some(receiver) = receiver_guard.as_mut() {
            match receiver.try_recv() {
                Ok(message) => {
                    self.update_activity().await;
                    Some(message)
                }
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    warn!("Message receiver disconnected for session {}", self.session_id);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Receive a message from this session (blocking)
    pub async fn receive_message(&self) -> Option<ProtocolMessage> {
        let mut receiver_guard = self.message_receiver.write().await;
        if let Some(receiver) = receiver_guard.as_mut() {
            match receiver.recv().await {
                Some(message) => {
                    self.update_activity().await;
                    Some(message)
                }
                None => {
                    warn!("Message receiver closed for session {}", self.session_id);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Send heartbeat message
    pub async fn send_heartbeat(&self) -> ProtocolResult<()> {
        let mut sequence = self.heartbeat_sequence.write().await;
        *sequence += 1;

        let payload = messages::MessagePayload::Heartbeat(messages::HeartbeatPayload {
            sequence_number: *sequence,
        });

        let message = ProtocolMessage::new(messages::MessageType::Heartbeat, payload)
            .with_session(self.session_id.clone());

        self.send_message(message).await
    }

    /// Handle incoming heartbeat
    pub async fn handle_heartbeat(&self, sequence: u64) -> ProtocolResult<()> {
        debug!("Received heartbeat {} for session {}", sequence, self.session_id);
        self.update_activity().await;

        // Send pong response
        let message = ProtocolMessage::new(messages::MessageType::Pong, messages::MessagePayload::Pong)
            .with_session(self.session_id.clone());

        self.send_message(message).await
    }

    /// Send error message
    pub async fn send_error(&self, error_code: u32, error_message: String) -> ProtocolResult<()> {
        let payload = messages::MessagePayload::Error(messages::ErrorPayload {
            error_code,
            error_message,
            details: None,
        });

        let message = ProtocolMessage::new(messages::MessageType::Error, payload)
            .with_session(self.session_id.clone());

        self.send_message(message).await
    }

    /// Close the session
    pub async fn close(&self) -> ProtocolResult<()> {
        info!("Closing session {}", self.session_id);

        // Send goodbye message
        let payload = messages::MessagePayload::Goodbye(messages::GoodbyePayload {
            reason: "Session closed by server".to_string(),
            code: 1000, // Normal closure
        });

        let message = ProtocolMessage::new(messages::MessageType::Goodbye, payload)
            .with_session(self.session_id.clone());

        let _ = self.send_message(message).await; // Ignore send errors during close

        // Set state to closing
        self.set_state(SessionState::Closing).await;

        // Close message channels
        {
            let mut receiver_guard = self.message_receiver.write().await;
            *receiver_guard = None;
        }

        // Set final state
        self.set_state(SessionState::Closed).await;

        Ok(())
    }

    /// Get session statistics
    pub async fn stats(&self) -> SessionStats {
        SessionStats {
            session_id: self.session_id.clone(),
            state: self.state().await,
            peer_id: self.peer_info.peer_id.clone(),
            last_activity: self.last_activity().await,
            is_authenticated: self.is_authenticated(),
            message_queue_size: 0, // TODO: Implement queue size tracking
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub session_id: String,
    pub state: SessionState,
    pub peer_id: String,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub is_authenticated: bool,
    pub message_queue_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let peer_info = PeerInfo {
            peer_id: "test-peer".to_string(),
            peer_name: "Test Peer".to_string(),
            address: NetworkAddress::localhost(8080),
            capabilities: vec!["video".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        let config = ProtocolConfig::default();
        let session = ProtocolSession::new("test-session".to_string(), peer_info, config);

        assert_eq!(session.session_id(), "test-session");
        assert!(!session.is_authenticated());
        assert!(!session.is_active().await);
    }

    #[tokio::test]
    async fn test_session_state_management() {
        let peer_info = PeerInfo {
            peer_id: "test-peer".to_string(),
            peer_name: "Test Peer".to_string(),
            address: NetworkAddress::localhost(8080),
            capabilities: vec!["video".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        let config = ProtocolConfig::default();
        let session = ProtocolSession::new("test-session".to_string(), peer_info, config);

        assert_eq!(session.state().await, SessionState::Connecting);

        session.set_state(SessionState::Active).await;
        assert_eq!(session.state().await, SessionState::Active);
        assert!(session.is_active().await);
    }

    #[tokio::test]
    async fn test_session_authentication() {
        let mut peer_info = PeerInfo {
            peer_id: "test-peer".to_string(),
            peer_name: "Test Peer".to_string(),
            address: NetworkAddress::localhost(8080),
            capabilities: vec!["video".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        let config = ProtocolConfig::default();
        let mut session = ProtocolSession::new("test-session".to_string(), peer_info, config);

        assert!(!session.is_authenticated());

        session.set_authenticated(true);
        assert!(session.is_authenticated());
    }
}
