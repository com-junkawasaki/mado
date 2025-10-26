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

//! # Soft KVM Protocol Plugin
//!
//! Tauri plugin for KVM protocol handling

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use soft_kvm_protocol::{ProtocolManager, ProtocolConfig, ProtocolResult, session::PeerInfo, messages::{MessageType, MessagePayload, ProtocolMessage}};
use soft_kvm_core::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolPluginConfig {
    pub version: String,
    pub max_message_size: usize,
    pub heartbeat_interval: u64,
    pub session_timeout: u64,
    pub compression_enabled: bool,
}

impl Default for ProtocolPluginConfig {
    fn default() -> Self {
        ProtocolPluginConfig {
            version: "1.0.0".to_string(),
            max_message_size: 1024 * 1024,
            heartbeat_interval: 30,
            session_timeout: 300,
            compression_enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolStatus {
    pub is_initialized: bool,
    pub active_sessions: usize,
    pub total_sessions: u64,
}

struct ProtocolPluginState {
    manager: Option<ProtocolManager>,
    config: Option<ProtocolPluginConfig>,
    session_counter: u64,
}

impl Default for ProtocolPluginState {
    fn default() -> Self {
        ProtocolPluginState {
            manager: None,
            config: None,
            session_counter: 0,
        }
    }
}

/// Initialize protocol
#[tauri::command]
async fn init_protocol(
    config: ProtocolPluginConfig,
    state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>,
) -> Result<String, String> {
    println!("Initializing protocol with config: {:?}", config);

    let mut plugin_state = state.write().await;

    // Convert plugin config to protocol config
    let protocol_config = ProtocolConfig {
        version: config.version.clone(),
        max_message_size: config.max_message_size,
        heartbeat_interval: config.heartbeat_interval,
        session_timeout: config.session_timeout,
        compression_enabled: config.compression_enabled,
    };

    // Create protocol manager
    let manager = ProtocolManager::new(protocol_config);

    // Start the manager
    manager.start().await
        .map_err(|e| format!("Failed to start protocol manager: {}", e))?;

    plugin_state.manager = Some(manager);
    plugin_state.config = Some(config);

    Ok("Protocol initialized successfully".to_string())
}

/// Start protocol server
#[tauri::command]
async fn start_protocol_server(
    address: String,
    state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>,
) -> Result<String, String> {
    let plugin_state = state.read().await;

    if let Some(manager) = &plugin_state.manager {
        // Parse address
        let socket_addr: std::net::SocketAddr = address.parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        // Start WebSocket server (simplified implementation)
        // In practice, this would start a WebSocket server using the transport layer
        println!("Starting protocol server on {}", socket_addr);

        Ok(format!("Protocol server started on {}", socket_addr))
    } else {
        Err("Protocol not initialized".to_string())
    }
}

/// Connect to protocol server
#[tauri::command]
async fn connect_to_protocol_server(
    address: String,
    state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>,
) -> Result<String, String> {
    let plugin_state = state.read().await;

    if let Some(manager) = &plugin_state.manager {
        // Parse address
        let socket_addr: std::net::SocketAddr = address.parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        drop(plugin_state); // Release read lock

        // Create session with write lock
        let mut plugin_state = state.write().await;
        plugin_state.session_counter += 1;
        let session_id = format!("session-{}", plugin_state.session_counter);

        let peer_info = PeerInfo {
            peer_id: "client".to_string(),
            peer_name: "Soft KVM Client".to_string(),
            address: NetworkAddress {
                ip: socket_addr.ip().to_string(),
                port: socket_addr.port() as u16,
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        // Re-acquire read lock for manager
        let plugin_state_read = state.read().await;
        if let Some(manager) = &plugin_state_read.manager {
            manager.create_session(session_id.clone(), peer_info).await
                .map_err(|e| format!("Failed to create session: {}", e))?;
        }

        // Connect to server (simplified implementation)
        // In practice, this would establish WebSocket connection
        println!("Connecting to protocol server at {}", socket_addr);

        Ok(format!("Connected to server, session: {}", session_id))
    } else {
        Err("Protocol not initialized".to_string())
    }
}

/// Send protocol message
#[tauri::command]
async fn send_protocol_message(
    session_id: String,
    message_type: String,
    payload: serde_json::Value,
    state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>,
) -> Result<String, String> {
    let plugin_state = state.read().await;

    if let Some(manager) = &plugin_state.manager {
        // Convert message type string to enum
        let msg_type = match message_type.as_str() {
            "hello" => MessageType::Hello,
            "heartbeat" => MessageType::Heartbeat,
            "video_start" => MessageType::VideoStart,
            "input_event" => MessageType::InputEvent,
            _ => return Err(format!("Unknown message type: {}", message_type)),
        };

        // Create message payload (simplified)
        let message_payload = match msg_type {
            MessageType::Hello => MessagePayload::Hello(soft_kvm_protocol::messages::HelloPayload {
                protocol_version: "1.0.0".to_string(),
                client_info: soft_kvm_protocol::messages::ClientInfo {
                    client_id: "client".to_string(),
                    client_name: "Soft KVM Client".to_string(),
                    platform: "unknown".to_string(),
                    version: "1.0.0".to_string(),
                },
                capabilities: vec!["video".to_string(), "input".to_string()],
            }),
            MessageType::Heartbeat => MessagePayload::Heartbeat(soft_kvm_protocol::messages::HeartbeatPayload {
                sequence_number: 1,
            }),
            _ => return Err(format!("Unsupported message type for sending: {}", message_type)),
        };

        let message = ProtocolMessage::new(msg_type, message_payload)
            .with_session(session_id.clone());

        manager.send_message(&session_id, message).await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(format!("Message sent to session {}", session_id))
    } else {
        Err("Protocol not initialized".to_string())
    }
}

/// Get protocol status
#[tauri::command]
async fn get_protocol_status(state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>) -> Result<ProtocolStatus, String> {
    let plugin_state = state.read().await;

    let is_initialized = plugin_state.manager.is_some();
    let active_sessions = if let Some(manager) = &plugin_state.manager {
        manager.active_sessions().await
    } else {
        0
    };

    Ok(ProtocolStatus {
        is_initialized,
        active_sessions,
        total_sessions: plugin_state.session_counter,
    })
}

/// Shutdown protocol
#[tauri::command]
async fn shutdown_protocol(state: tauri::State<'_, Arc<RwLock<ProtocolPluginState>>>) -> Result<String, String> {
    let mut plugin_state = state.write().await;

    if let Some(manager) = plugin_state.manager.take() {
        manager.stop().await
            .map_err(|e| format!("Failed to stop protocol manager: {}", e))?;

        Ok("Protocol shutdown successfully".to_string())
    } else {
        Ok("Protocol was not running".to_string())
    }
}

/// Initialize the protocol plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-protocol")
        .invoke_handler(tauri::generate_handler![
            init_protocol,
            start_protocol_server,
            connect_to_protocol_server,
            send_protocol_message,
            get_protocol_status,
            shutdown_protocol,
        ])
        .setup(|app, _app_handle| {
            // Initialize protocol plugin state
            let state = Arc::new(RwLock::new(ProtocolPluginState::default()));
            app.manage(state);
            Ok(())
        })
        .build()
}
