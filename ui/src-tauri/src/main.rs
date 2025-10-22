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

//! # Soft KVM Tauri UI
//!
//! Main application that integrates all plugins

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde_json;
use tauri::{AppHandle, Manager, State};
use std::sync::Arc;
use tokio::sync::RwLock;

// Import plugins
use soft_kvm_plugin_input::init as input_plugin_init;
// Protocol Manager is integrated directly into UI AppState
// use soft_kvm_plugin_protocol::init as protocol_plugin_init;
use soft_kvm_plugin_service::init as service_plugin_init;
use soft_kvm_plugin_security::init as security_plugin_init;
// use soft_kvm_plugin_discovery::init as discovery_plugin_init;

// Internal crates
use soft_kvm_core::*;
use soft_kvm_discovery::*;
use soft_kvm_protocol::{ProtocolManager, ProtocolConfig};

// アプリケーションのグローバル状態
#[derive(Default)]
struct AppState {
    connection_status: String,
    discovery: Option<ServiceResolver>,
    protocol_manager: Option<ProtocolManager>,
    protocol_session_id: Option<String>,
}

// UI固有のTauriコマンド

/// 利用可能なサーバーを取得（簡易版）
#[tauri::command]
async fn get_available_servers_ui(_state: State<'_, Arc<RwLock<AppState>>>) -> Result<Vec<serde_json::Value>, String> {
    // 簡易実装: サンプルサーバーを返す
    let servers = vec![
        serde_json::json!({
            "id": "server-001",
            "name": "My KVM Server",
            "address": "192.168.1.100:8080",
            "service_type": "Server",
            "is_expired": false,
        }),
        serde_json::json!({
            "id": "server-002",
            "name": "Office KVM",
            "address": "192.168.1.101:8080",
            "service_type": "Server",
            "is_expired": false,
        }),
    ];

    Ok(servers)
}

/// サーバーに接続（簡易版）
#[tauri::command]
async fn connect_to_server_ui(
    server_address: String,
    state: State<'_, Arc<RwLock<AppState>>>,
    _app_handle: AppHandle,
) -> Result<String, String> {
    println!("Connecting to server: {}", server_address);

    // 接続状態を更新
    {
        let mut app_state = state.write().await;
        app_state.connection_status = format!("Connected to {}", server_address);
    }

    // 接続成功をシミュレート
    Ok(format!("Connected to {}", server_address))
}

/// サーバーから切断（簡易版）
#[tauri::command]
async fn disconnect_from_server_ui(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    println!("Disconnected from server");

    // 接続状態を更新
    {
        let mut app_state = state.write().await;
        app_state.connection_status = "Disconnected".to_string();
    }

    Ok("Disconnected".to_string())
}

/// メトリクスを取得（簡易版）
#[tauri::command]
async fn get_metrics_ui(_state: State<'_, Arc<RwLock<AppState>>>) -> Result<serde_json::Value, String> {
    // サンプルメトリクスを返す
    let metrics = serde_json::json!({
        "cpu_usage": 15.5,
        "memory_usage": 256.0,
        "network_bytes": 1024000,
        "active_connections": 1,
        "video_latency_p99": 42.3,
        "input_latency_p99": 8.7,
    });

    Ok(metrics)
}

/// 設定を取得（簡易版）
#[tauri::command]
async fn get_settings_ui() -> Result<serde_json::Value, String> {
    let settings = serde_json::json!({
        "video": {
            "resolution": "1920x1080",
            "fps": 30,
            "quality": "balanced",
        },
        "input": {
            "keyboard_enabled": true,
            "mouse_enabled": true,
        },
        "network": {
            "discovery_enabled": true,
            "auto_connect": false,
        },
    });

    Ok(settings)
}

/// 設定を更新（簡易版）
#[tauri::command]
async fn update_settings_ui(settings: serde_json::Value) -> Result<String, String> {
    println!("Updating settings: {:?}", settings);
    // TODO: 設定の永続化を実装
    Ok("Settings updated".to_string())
}

/// Protocol Managerを初期化
#[tauri::command]
async fn init_protocol_ui(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    println!("Initializing protocol manager via UI");

    let mut app_state = state.write().await;

    // Protocol Managerが既に初期化されている場合は何もしない
    if app_state.protocol_manager.is_some() {
        return Ok("Protocol manager already initialized".to_string());
    }

    // Protocol Managerを初期化
    let config = ProtocolConfig {
        version: "1.0.0".to_string(),
        max_message_size: 1024 * 1024, // 1MB
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    let manager = ProtocolManager::new(config);
    manager.start().await
        .map_err(|e| format!("Failed to start protocol manager: {}", e))?;

    app_state.protocol_manager = Some(manager);

    Ok("Protocol manager initialized successfully".to_string())
}

/// Protocolサーバーを開始
#[tauri::command]
async fn start_protocol_server_ui(
    address: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    println!("Starting protocol server on: {}", address);

    let app_state = state.read().await;

    if let Some(manager) = &app_state.protocol_manager {
        // サーバー開始はWebSocketトランスポート層の実装が必要
        // 現在はプレースホルダー実装
        println!("Protocol server start requested on {}", address);
        Ok(format!("Protocol server start requested on {}", address))
    } else {
        Err("Protocol manager not initialized".to_string())
    }
}

/// Protocolサーバーに接続
#[tauri::command]
async fn connect_to_protocol_server_ui(
    address: String,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    println!("Connecting to protocol server: {}", address);

    let mut app_state = state.write().await;

    if let Some(manager) = &app_state.protocol_manager {
        // セッションIDを生成
        use uuid::Uuid;
        let session_id = format!("session-{}", Uuid::new_v4());

        // PeerInfoを作成
        let peer_info = soft_kvm_protocol::session::PeerInfo {
            peer_id: "client".to_string(),
            peer_name: "Soft KVM Client".to_string(),
            address: NetworkAddress {
                ip: address.split(':').next().unwrap_or("127.0.0.1").to_string(),
                port: address.split(':').nth(1).unwrap_or("8080").parse().unwrap_or(8080),
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
            authenticated: false,
            last_seen: chrono::Utc::now(),
        };

        // セッションを作成
        manager.create_session(session_id.clone(), peer_info).await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        // 接続状態を更新
        app_state.connection_status = format!("Connected to {} (session: {})", address, session_id);
        app_state.protocol_session_id = Some(session_id.clone());

        Ok(format!("Connected to {} with session {}", address, session_id))
    } else {
        Err("Protocol manager not initialized".to_string())
    }
}

/// Protocolメッセージを送信
#[tauri::command]
async fn send_protocol_message_ui(
    message_type: String,
    payload: serde_json::Value,
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    let app_state = state.read().await;
    let session_id = app_state.protocol_session_id.as_ref()
        .ok_or("No active protocol session")?;

    if let Some(manager) = &app_state.protocol_manager {
        println!("Sending {} message to session {}", message_type, session_id);

        // メッセージタイプをProtocolMessageTypeに変換
        use soft_kvm_protocol::messages::{MessageType, MessagePayload, ProtocolMessage};

        let msg_type = match message_type.as_str() {
            "hello" => MessageType::Hello,
            "heartbeat" => MessageType::Heartbeat,
            "video_start" => MessageType::VideoStart,
            "input_event" => MessageType::InputEvent,
            _ => return Err(format!("Unknown message type: {}", message_type)),
        };

        // ペイロードを作成（簡易実装）
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

        manager.send_message(session_id, message).await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(format!("Message sent to session {}", session_id))
    } else {
        Err("Protocol manager not initialized".to_string())
    }
}

/// Protocolステータスを取得
#[tauri::command]
async fn get_protocol_status_ui(state: State<'_, Arc<RwLock<AppState>>>) -> Result<serde_json::Value, String> {
    let app_state = state.read().await;

    let status = serde_json::json!({
        "is_initialized": app_state.protocol_manager.is_some(),
        "active_sessions": if let Some(manager) = &app_state.protocol_manager {
            manager.active_sessions().await
        } else {
            0
        },
        "current_session": app_state.protocol_session_id,
        "connection_status": app_state.connection_status,
    });

    Ok(status)
}

/// Protocolをシャットダウン
#[tauri::command]
async fn shutdown_protocol_ui(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    println!("Shutting down protocol");

    let mut app_state = state.write().await;

    if let Some(manager) = app_state.protocol_manager.take() {
        manager.stop().await
            .map_err(|e| format!("Failed to stop protocol manager: {}", e))?;

        app_state.connection_status = "Protocol shutdown".to_string();
        app_state.protocol_session_id = None;

        Ok("Protocol shutdown successfully".to_string())
    } else {
        Ok("Protocol was not running".to_string())
    }
}

/// 接続状態を取得
#[tauri::command]
async fn get_connection_status(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    let app_state = state.read().await;
    Ok(app_state.connection_status.clone())
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(AppState::default())))
        // Register plugins
        .plugin(input_plugin_init())
        // Protocol Manager is integrated directly into UI AppState
        // .plugin(protocol_plugin_init())
        .plugin(service_plugin_init())
        .plugin(security_plugin_init())
        // .plugin(discovery_plugin_init())
        // UI-specific commands
        .invoke_handler(tauri::generate_handler![
            get_available_servers_ui,
            connect_to_server_ui,
            disconnect_from_server_ui,
            get_metrics_ui,
            get_settings_ui,
            update_settings_ui,
            get_connection_status,
            // Protocol commands
            init_protocol_ui,
            start_protocol_server_ui,
            connect_to_protocol_server_ui,
            send_protocol_message_ui,
            get_protocol_status_ui,
            shutdown_protocol_ui,
        ]);

    println!("Soft KVM Tauri application initialized successfully!");
    println!("Note: GUI display is disabled for development - plugins are ready");
}
