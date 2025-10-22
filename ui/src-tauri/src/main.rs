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
use soft_kvm_plugin_service::init as service_plugin_init;
use soft_kvm_plugin_security::init as security_plugin_init;
use soft_kvm_plugin_discovery::init as discovery_plugin_init;

// Internal crates
use soft_kvm_core::*;
use soft_kvm_discovery::*;

// アプリケーションのグローバル状態
#[derive(Default)]
struct AppState {
    connection_status: String,
    discovery: Option<ServiceResolver>,
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
        .plugin(service_plugin_init())
        .plugin(security_plugin_init())
        .plugin(discovery_plugin_init())
        // UI-specific commands
        .invoke_handler(tauri::generate_handler![
            get_available_servers_ui,
            connect_to_server_ui,
            disconnect_from_server_ui,
            get_metrics_ui,
            get_settings_ui,
            update_settings_ui,
            get_connection_status,
        ]);

    println!("Soft KVM Tauri application initialized successfully!");
    println!("Note: GUI display is disabled for development - plugins are ready");
}
