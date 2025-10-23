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

//! Client message handler

use crate::{ClientConfig, ClientResult, ClientError, KvmClient};
use soft_kvm_core::*;
use soft_kvm_protocol::{messages::*, session::*, *};
use soft_kvm_platform::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Client message handler
pub struct ClientMessageHandler {
    config: ClientConfig,
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    platform_manager: Option<PlatformManager>,
    client: Arc<RwLock<Option<KvmClient>>>,
}

impl ClientMessageHandler {
    /// Create a new client message handler
    pub fn new(
        config: ClientConfig,
        protocol_manager: Arc<RwLock<ProtocolManager>>,
        platform_manager: Option<PlatformManager>,
    ) -> Self {
        ClientMessageHandler {
            config,
            protocol_manager,
            platform_manager,
            client: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the client reference (to avoid circular references)
    pub async fn set_client(&self, client: Arc<RwLock<KvmClient>>) {
        let mut client_ref = self.client.write().await;
        *client_ref = Some((*client.read().await).clone());
    }

    /// Handle incoming protocol message
    pub async fn handle_message(
        &mut self,
        message: ProtocolMessage,
        sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ClientResult<()> {
        debug!("Client handling message: {:?}", message.message_type());

        match message.payload {
            MessagePayload::Welcome(payload) => {
                self.handle_welcome(message, payload, sender).await
            }
            MessagePayload::AuthResponse(payload) => {
                self.handle_auth_response(message, payload, sender).await
            }
            MessagePayload::Heartbeat(payload) => {
                self.handle_heartbeat(message, payload).await
            }
            MessagePayload::Goodbye(payload) => {
                self.handle_goodbye(message, payload).await
            }
            MessagePayload::VideoFrame(payload) => {
                self.handle_video_frame(message, payload).await
            }
            _ => {
                warn!("Unhandled message type: {:?}", message.message_type());
                Ok(())
            }
        }
    }

    /// Handle welcome message
    async fn handle_welcome(
        &mut self,
        message: ProtocolMessage,
        payload: WelcomePayload,
        _sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ClientResult<()> {
        info!("Received welcome from server: {}", payload.server_info.server_name);

        // Store session ID
        if let Some(session_id) = payload.session_id {
            if let Some(client) = &*self.client.read().await {
                client.set_session_id(session_id).await;
            }
        }

        // Send authentication request if security is enabled
        if self.config.enable_security {
            if let Some(session_id) = payload.session_id {
                self.send_auth_request(session_id).await?;
            }
        } else {
            // Mark as authenticated
            info!("Security disabled, proceeding without authentication");
        }

        info!("Connected to server: {}", payload.server_info.server_name);
        Ok(())
    }

    /// Send authentication request
    async fn send_auth_request(&mut self, session_id: String) -> ClientResult<()> {
        let auth_payload = AuthRequestPayload {
            auth_method: "none".to_string(), // TODO: Implement proper authentication
            credentials: None,
            client_info: Some(ClientInfo {
                client_id: uuid::Uuid::new_v4().to_string(),
                client_name: self.config.client_name.clone(),
                platform: std::env::consts::OS.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        };

        let message = ProtocolMessage::new(
            MessageType::AuthRequest,
            MessagePayload::AuthRequest(auth_payload),
        ).with_session(session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Handle authentication response
    async fn handle_auth_response(
        &mut self,
        message: ProtocolMessage,
        payload: AuthResponsePayload,
        _sender: tokio::sync::mpsc::UnboundedSender<ProtocolMessage>,
    ) -> ClientResult<()> {
        if payload.success {
            info!("Authentication successful for session: {}", message.session_id);

            // Start input capture if platform is available
            if let Some(platform) = &mut self.platform_manager {
                platform.start_input_capture(default_input_config()).await?;
                info!("Input capture started on client");
            }
        } else {
            let error_msg = payload.error_message.unwrap_or_else(|| "Unknown authentication error".to_string());
            error!("Authentication failed: {}", error_msg);
            return Err(ClientError::Authentication(error_msg));
        }

        Ok(())
    }

    /// Handle heartbeat message
    async fn handle_heartbeat(
        &mut self,
        message: ProtocolMessage,
        payload: HeartbeatPayload,
    ) -> ClientResult<()> {
        debug!("Received heartbeat from server (seq: {})", payload.sequence_number);

        // Send heartbeat response
        let session_id = message.session_id().unwrap_or(&"".to_string()).clone();
        let pong_message = ProtocolMessage::new(
            MessageType::Pong,
            MessagePayload::Pong,
        ).with_session(session_id);

        let mut protocol_manager = self.protocol_manager.write().await;
        protocol_manager.send_client_message(pong_message).await?;
        drop(protocol_manager);

        Ok(())
    }

    /// Handle goodbye message
    async fn handle_goodbye(
        &mut self,
        message: ProtocolMessage,
        payload: GoodbyePayload,
    ) -> ClientResult<()> {
        info!("Received goodbye from server: {}", payload.reason);

        // Stop input capture
        if let Some(platform) = &mut self.platform_manager {
            platform.stop_input_capture().await?;
        }

        // Clear session ID
        if let Some(client) = &*self.client.read().await {
            let mut session_id = client.session_id.write().await;
            *session_id = None;
        }

        Ok(())
    }

    /// Handle video frame (for future video streaming)
    async fn handle_video_frame(
        &mut self,
        _message: ProtocolMessage,
        _payload: VideoFramePayload,
    ) -> ClientResult<()> {
        // TODO: Implement video frame handling
        debug!("Received video frame (not yet implemented)");
        Ok(())
    }
}
