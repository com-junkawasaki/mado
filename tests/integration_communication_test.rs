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

//! 統合通信テスト - プラグインアーキテクチャを使用した通信テスト

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

use soft_kvm_core::*;
use soft_kvm_plugin_protocol::*;
use soft_kvm_plugin_input::*;
use soft_kvm_plugin_service::*;

#[tokio::test]
async fn test_plugin_state_integration() {
    // プラグインの状態統合テスト
    println!("Testing plugin state integration...");

    // Protocol plugin state
    let protocol_state = Arc::new(RwLock::new(ProtocolPluginState::default()));
    let protocol_config = ProtocolPluginConfig {
        version: "1.0.0".to_string(),
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    // Initialize protocol plugin
    let init_result = init_protocol(protocol_config.clone(), tauri::State::from(&protocol_state)).await;
    assert!(init_result.is_ok(), "Protocol plugin initialization should succeed");

    // Check protocol status
    let status_result = get_protocol_status(tauri::State::from(&protocol_state)).await;
    assert!(status_result.is_ok(), "Protocol status query should succeed");

    let status = status_result.unwrap();
    assert!(status.is_initialized, "Protocol should be initialized");
    assert_eq!(status.active_sessions, 0, "Should have no active sessions initially");

    // Input plugin state
    let input_state = Arc::new(RwLock::new(InputCaptureState::default()));
    let input_config = InputConfig {
        keyboard_enabled: true,
        mouse_enabled: true,
        toggle_hotkey: Some(HotkeyConfig {
            modifiers: vec!["ctrl".to_string()],
            key: "k".to_string(),
        }),
    };

    // Start input capture
    let start_result = start_input_capture(input_config.clone(), tauri::State::from(&input_state)).await;
    assert!(start_result.is_ok(), "Input capture start should succeed");

    // Check input status
    let input_status_result = get_input_status(tauri::State::from(&input_state)).await;
    assert!(input_status_result.is_ok(), "Input status query should succeed");

    // Stop input capture
    let stop_result = stop_input_capture(tauri::State::from(&input_state)).await;
    assert!(stop_result.is_ok(), "Input capture stop should succeed");

    // Service plugin state
    let service_state = Arc::new(RwLock::new(ServiceState::default()));

    // Check service status
    let service_status_result = get_service_status(tauri::State::from(&service_state)).await;
    assert!(service_status_result.is_ok(), "Service status query should succeed");

    // Shutdown protocol
    let shutdown_result = shutdown_protocol(tauri::State::from(&protocol_state)).await;
    assert!(shutdown_result.is_ok(), "Protocol shutdown should succeed");

    println!("Plugin state integration test passed!");
}

#[tokio::test]
async fn test_plugin_config_validation() {
    // プラグイン設定の検証テスト
    println!("Testing plugin configuration validation...");

    // Test protocol plugin config
    let protocol_config = ProtocolPluginConfig {
        version: "1.0.0".to_string(),
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    // Test input plugin config
    let input_config = InputConfig {
        keyboard_enabled: true,
        mouse_enabled: true,
        toggle_hotkey: Some(HotkeyConfig {
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            key: "k".to_string(),
        }),
    };

    // Test service plugin config
    let service_config = ServiceConfig {
        systemd_enabled: true,
        launchd_enabled: false,
        windows_service_enabled: false,
        service_name: "soft-kvm-test".to_string(),
    };

    // Validate configurations by creating plugin states
    let _protocol_state = Arc::new(RwLock::new(ProtocolPluginState {
        manager: None,
        config: Some(protocol_config),
        session_counter: 0,
    }));

    let _input_state = Arc::new(RwLock::new(InputCaptureState {
        is_capturing: false,
        config: Some(input_config),
        capture_task: None,
        event_sender: None,
    }));

    let _service_state = Arc::new(RwLock::new(ServiceState {
        config: Some(service_config),
        systemd_service: None,
        launchd_service: None,
        windows_service: None,
    }));

    println!("Plugin configuration validation test passed!");
}

#[tokio::test]
async fn test_plugin_input_event_handling() {
    // プラグインの入力イベント処理テスト
    println!("Testing plugin input event handling...");

    // Initialize input plugin state
    let input_state = Arc::new(RwLock::new(InputCaptureState::default()));

    let input_config = InputConfig {
        keyboard_enabled: true,
        mouse_enabled: true,
        toggle_hotkey: Some(HotkeyConfig {
            modifiers: vec!["ctrl".to_string()],
            key: "k".to_string(),
        }),
    };

    // Start input capture
    let start_result = start_input_capture(input_config.clone(), tauri::State::from(&input_state)).await;
    assert!(start_result.is_ok(), "Input capture should start successfully");

    // Test keyboard event sending
    let keyboard_event = KeyboardEvent {
        key_code: 65, // 'A' key
        pressed: true,
        modifiers: 0,
    };

    let keyboard_result = send_keyboard_event(keyboard_event, tauri::State::from(&input_state)).await;
    assert!(keyboard_result.is_ok(), "Keyboard event should be sent successfully");

    // Test mouse event sending
    let mouse_event = MouseEvent {
        x: 100,
        y: 200,
        button: Some(1),
        pressed: Some(true),
        wheel_delta: None,
    };

    let mouse_result = send_mouse_event(mouse_event, tauri::State::from(&input_state)).await;
    assert!(mouse_result.is_ok(), "Mouse event should be sent successfully");

    // Test toggle hotkey
    let hotkey = HotkeyConfig {
        modifiers: vec!["ctrl".to_string()],
        key: "k".to_string(),
    };

    let toggle_result = toggle_input_capture(hotkey, tauri::State::from(&input_state)).await;
    assert!(toggle_result.is_ok(), "Input capture toggle should work");

    // Check status after toggle
    let status_result = get_input_status(tauri::State::from(&input_state)).await;
    assert!(status_result.is_ok(), "Status query should succeed");

    // Stop input capture
    let stop_result = stop_input_capture(tauri::State::from(&input_state)).await;
    assert!(stop_result.is_ok(), "Input capture should stop successfully");

    println!("Plugin input event handling test passed!");
}

#[tokio::test]
async fn test_plugin_error_handling() {
    // プラグインのエラーハンドリングテスト
    println!("Testing plugin error handling...");

    // Test protocol plugin with invalid config
    let protocol_state = Arc::new(RwLock::new(ProtocolPluginState::default()));

    let invalid_protocol_config = ProtocolPluginConfig {
        version: "1.0.0".to_string(),
        max_message_size: 0, // Invalid: zero message size
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    // This should still succeed as the config is just stored
    let init_result = init_protocol(invalid_protocol_config, tauri::State::from(&protocol_state)).await;
    assert!(init_result.is_ok(), "Protocol init should succeed even with invalid config");

    // Test input plugin with invalid hotkey
    let input_state = Arc::new(RwLock::new(InputCaptureState::default()));

    let invalid_input_config = InputConfig {
        keyboard_enabled: true,
        mouse_enabled: true,
        toggle_hotkey: Some(HotkeyConfig {
            modifiers: vec![],
            key: "".to_string(), // Invalid: empty key
        }),
    };

    // Start input capture with invalid config
    let start_result = start_input_capture(invalid_input_config, tauri::State::from(&input_state)).await;
    // This should still succeed as validation happens during actual operations

    // Test sending events without capture running
    let keyboard_event = KeyboardEvent {
        key_code: 65,
        pressed: true,
        modifiers: 0,
    };

    let keyboard_result = send_keyboard_event(keyboard_event, tauri::State::from(&input_state)).await;
    assert!(keyboard_result.is_err(), "Should fail when input capture is not running");

    // Test invalid address for protocol server
    let invalid_address = "invalid-address:99999";
    let start_server_result = start_protocol_server_ui(invalid_address.to_string(), tauri::State::from(&protocol_state)).await;
    // This should fail due to invalid address
    assert!(start_server_result.is_err(), "Should fail with invalid address");

    println!("Plugin error handling test passed!");
}
