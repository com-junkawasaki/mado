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

//! Server message handler

use crate::{ServerConfig, ServerResult, ServerError};
use soft_kvm_core::*;
use soft_kvm_protocol::{messages::*, session::*, *};
use soft_kvm_platform::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Server message handler
pub struct ServerMessageHandler {
    config: ServerConfig,
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    platform_manager: Option<PlatformManager>,
    active_sessions: Arc<RwLock<std::collections::HashMap<String, ClientSession>>>,
}

/// Client session information
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: String,
    pub client_info: ClientInfo,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl ServerMessageHandler {
    /// Create a new server message handler
    pub fn new(
        config: ServerConfig,
        protocol_manager: Arc<RwLock<ProtocolManager>>,
        platform_manager: Option<PlatformManager>,
    ) -> Self {
        ServerMessageHandler {
            config,
            protocol_manager,
            platform_manager,
            active_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Handle incoming protocol message
    pub async fn handle_message(
        &mut self,
        message: ProtocolMessage,
        sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ServerResult<()> {
        debug!("Server handling message: {:?}", message.message_type());

        match message.payload {
            Some(MessagePayload::Hello(payload)) => {
                self.handle_hello(message, payload, sender).await
            }
            Some(MessagePayload::AuthRequest(payload)) => {
                self.handle_auth_request(message, payload, sender).await
            }
            Some(MessagePayload::Heartbeat(payload)) => {
                self.handle_heartbeat(message, payload).await
            }
            Some(MessagePayload::Goodbye(payload)) => {
                self.handle_goodbye(message, payload).await
            }
            Some(MessagePayload::InputEvent(payload)) => {
                self.handle_input_event(message, payload).await
            }
            _ => {
                warn!("Unhandled message type: {:?}", message.message_type());
                Ok(())
            }
        }
    }

    /// Handle hello message
    async fn handle_hello(
        &mut self,
        message: ProtocolMessage,
        payload: HelloPayload,
        sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ServerResult<()> {
        info!("Received hello from client: {}", payload.client_info.client_name);

        // Create session
        let session_id = format!("server-session-{}", uuid::Uuid::new_v4());
        let peer_info = session::PeerInfo {
            peer_id: payload.client_info.client_id.clone(),
            peer_name: payload.client_info.client_name.clone(),
            address: NetworkAddress {
                ip: "127.0.0.1".to_string(), // TODO: Get from connection
                port: 0,
            },
            capabilities: payload.capabilities.clone(),
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        let protocol_config = self.protocol_manager.read().await.config().clone();
        let session = session::ProtocolSession::new(
            session_id.clone(),
            peer_info,
            protocol_config,
        );

        // Store session
        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.create_session(session_id.clone(), session.peer_info().clone()).await?;
        drop(protocol_manager);

        // Store client session
        let client_session = ClientSession {
            session_id: session_id.clone(),
            client_info: payload.client_info.clone(),
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        let mut active_sessions = self.active_sessions.write().await;
        active_sessions.insert(session_id.clone(), client_session);
        drop(active_sessions);

        // Send welcome response
        let welcome_payload = WelcomePayload {
            server_info: ServerInfo {
                server_id: "soft-kvm-server".to_string(),
                server_name: self.config.server_name.clone(),
                platform: std::env::consts::OS.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec!["video".to_string(), "input".to_string()],
            },
            session_id: session_id.clone(),
            supported_protocols: vec![PROTOCOL_VERSION.to_string()],
            server_capabilities: vec!["video".to_string(), "input".to_string()],
        };

        let response = ProtocolMessage::new(
            MessageType::Welcome,
            MessagePayload::Welcome(welcome_payload),
        ).with_session(session_id);

        sender.send(response)
            .map_err(|e| ServerError::Generic(format!("Failed to send welcome: {}", e)))?;

        info!("Sent welcome to client: {}", payload.client_info.client_name);
        Ok(())
    }

    /// Handle authentication request
    async fn handle_auth_request(
        &mut self,
        message: ProtocolMessage,
        _payload: AuthRequestPayload,
        sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ServerResult<()> {
        // TODO: Implement proper authentication
        // For now, always accept

        let auth_response = AuthResponsePayload {
            success: true,
            session_token: Some("dummy-token".to_string()),
            error_message: None,
            client_capabilities: Some(vec!["video".to_string(), "input".to_string()]),
        };

        let response = ProtocolMessage::new(
            MessageType::AuthResponse,
            MessagePayload::AuthResponse(auth_response),
        ).with_session(message.session_id);

        sender.send(response)
            .map_err(|e| ServerError::Generic(format!("Failed to send auth response: {}", e)))?;

        // Mark session as authenticated
        let mut active_sessions = self.active_sessions.write().await;
        if let Some(session) = active_sessions.get_mut(&message.session_id) {
            session.last_activity = chrono::Utc::now();
        }

        info!("Authentication successful for session: {}", message.session_id);
        Ok(())
    }

    /// Handle heartbeat message
    async fn handle_heartbeat(
        &mut self,
        message: ProtocolMessage,
        payload: HeartbeatPayload,
    ) -> ServerResult<()> {
        debug!("Received heartbeat from session: {} (seq: {})",
               message.session_id, payload.sequence_number);

        // Update last activity
        let mut active_sessions = self.active_sessions.write().await;
        if let Some(session) = active_sessions.get_mut(&message.session_id) {
            session.last_activity = chrono::Utc::now();
        }

        Ok(())
    }

    /// Handle goodbye message
    async fn handle_goodbye(
        &mut self,
        message: ProtocolMessage,
        payload: GoodbyePayload,
    ) -> ServerResult<()> {
        info!("Received goodbye from session: {} (reason: {})",
              message.session_id, payload.reason);

        // Remove session
        let mut active_sessions = self.active_sessions.write().await;
        active_sessions.remove(&message.session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.remove_session(&message.session_id).await?;

        Ok(())
    }

    /// Handle input event
    async fn handle_input_event(
        &mut self,
        message: ProtocolMessage,
        payload: InputEventPayload,
    ) -> ServerResult<()> {
        debug!("Received input event from session: {}", message.session_id);

        // Forward input events to platform
        if let Some(platform) = &mut self.platform_manager {
            match payload.event_type() {
                "keyboard" => {
                    if let Some(keyboard_event) = payload.keyboard_event {
                        platform.send_keyboard_event(keyboard_event).await?;
                    }
                }
                "mouse" => {
                    if let Some(mouse_event) = payload.mouse_event {
                        platform.send_mouse_event(mouse_event).await?;
                    }
                }
                _ => {
                    warn!("Unknown input event type: {}", payload.event_type);
                }
            }
        }

        // Update last activity
        let mut active_sessions = self.active_sessions.write().await;
        if let Some(session) = active_sessions.get_mut(&message.session_id) {
            session.last_activity = chrono::Utc::now();
        }

        Ok(())
    }

    /// Get active sessions
    pub async fn get_active_sessions(&self) -> Vec<ClientSession> {
        let active_sessions = self.active_sessions.read().await;
        active_sessions.values().cloned().collect()
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        let active_sessions = self.active_sessions.read().await;
        active_sessions.len()
    }
}
