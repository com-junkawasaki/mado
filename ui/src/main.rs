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
//! クロスプラットフォームGUIアプリケーション

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use soft_kvm_core::*;
use soft_kvm_discovery::*;
use soft_kvm_monitoring::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{AppHandle, Manager, State};

// アプリケーションのグローバル状態
#[derive(Default)]
struct AppState {
    discovery: Option<ServiceResolver>,
    metrics_collector: MetricsCollector,
}

// Tauriコマンドの実装

/// 利用可能なサーバーを取得
#[tauri::command]
async fn get_available_servers(state: State<'_, Arc<RwLock<AppState>>>) -> Result<Vec<serde_json::Value>, String> {
    let app_state = state.read().await;

    if let Some(discovery) = &app_state.discovery {
        let services = discovery.get_available_services().await;
        let result = services.into_iter().map(|service| {
            serde_json::json!({
                "id": service.id.0.to_string(),
                "name": service.name,
                "address": format!("{}:{}", service.address.ip, service.address.port),
                "service_type": format!("{:?}", service.service_type),
                "is_expired": service.is_expired(),
            })
        }).collect();
        Ok(result)
    } else {
        Ok(vec![])
    }
}

/// サーバーに接続
#[tauri::command]
async fn connect_to_server(
    server_address: String,
    state: State<'_, Arc<RwLock<AppState>>>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // TODO: 実際の接続ロジックを実装
    info!("Connecting to server: {}", server_address);

    // 接続成功をシミュレート
    Ok(format!("Connected to {}", server_address))
}

/// サーバーから切断
#[tauri::command]
async fn disconnect_from_server(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    // TODO: 切断ロジックを実装
    info!("Disconnected from server");
    Ok("Disconnected".to_string())
}

/// メトリクスを取得
#[tauri::command]
async fn get_metrics(state: State<'_, Arc<RwLock<AppState>>>) -> Result<serde_json::Value, String> {
    let app_state = state.read().await;

    // TODO: 実際のメトリクス収集を実装
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

/// 設定を取得
#[tauri::command]
async fn get_settings() -> Result<serde_json::Value, String> {
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

/// 設定を更新
#[tauri::command]
async fn update_settings(settings: serde_json::Value) -> Result<String, String> {
    info!("Updating settings: {:?}", settings);
    // TODO: 設定の永続化を実装
    Ok("Settings updated".to_string())
}

/// アプリケーションを初期化
fn init_app() -> Result<(), Box<dyn std::error::Error>> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    Ok(())
}

fn main() {
    if let Err(e) = init_app() {
        eprintln!("Failed to initialize application: {}", e);
        std::process::exit(1);
    }

    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            get_available_servers,
            connect_to_server,
            disconnect_from_server,
            get_metrics,
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
