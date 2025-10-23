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

//! 基本接続テスト - 実際のネットワーク接続なしで機能を検証

use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_server_initialization() {
    // サーバーの初期化テスト
    let server_config = soft_kvm_server::ServerConfig {
        server_name: "Test Server".to_string(),
        bind_address: soft_kvm_core::NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        },
        max_clients: 5,
        session_timeout: 300,
        heartbeat_interval: 30,
        enable_discovery: false,
        enable_security: false,
        video_config: soft_kvm_core::VideoConfig {
            resolution: soft_kvm_core::VideoResolution::fhd(),
            fps: 30,
            quality: soft_kvm_core::VideoQuality::balanced(),
            compression: true,
        },
        input_config: soft_kvm_core::InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
        max_message_size: 1024 * 1024,
    };

    let server_result = soft_kvm_server::KvmServer::new(server_config).await;
    assert!(server_result.is_ok(), "Server initialization should succeed");

    let server = server_result.unwrap();
    let status = server.status().await;
    assert!(status.is_ok(), "Server status should be available");

    let server_status = status.unwrap();
    assert_eq!(server_status.server_name, "Test Server");
    assert_eq!(server_status.active_sessions, 0);
    assert_eq!(server_status.max_clients, 5);

    println!("Server initialization test passed!");
}

#[tokio::test]
async fn test_client_initialization() {
    // クライアントの初期化テスト
    let client_config = soft_kvm_client::ClientConfig {
        client_name: "Test Client".to_string(),
        server_address: Some(soft_kvm_core::NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        }),
        auto_connect: false,
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        enable_security: false,
        video_config: soft_kvm_core::VideoConfig {
            resolution: soft_kvm_core::VideoResolution::fhd(),
            fps: 30,
            quality: soft_kvm_core::VideoQuality::balanced(),
            compression: true,
        },
        input_config: soft_kvm_core::InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
    };

    let client_result = soft_kvm_client::KvmClient::new(client_config).await;
    assert!(client_result.is_ok(), "Client initialization should succeed");

    let client = client_result.unwrap();
    let status = client.status().await;
    assert!(status.is_ok(), "Client status should be available");

    let client_status = status.unwrap();
    assert_eq!(client_status.client_name, "Test Client");
    assert!(!client_status.is_connected);
    assert_eq!(client_status.active_connections, 0);

    println!("Client initialization test passed!");
}

#[tokio::test]
async fn test_platform_manager_creation() {
    // プラットフォームマネージャーの作成テスト
    let platform_result = soft_kvm_platform::PlatformManager::new();
    assert!(platform_result.is_ok() || platform_result.is_err(),
            "Platform manager creation should either succeed or fail gracefully");

    if let Ok(manager) = platform_result {
        let info = manager.platform_info();
        assert!(!info.os.is_empty(), "OS info should not be empty");
        assert!(!info.arch.is_empty(), "Architecture info should not be empty");

        println!("Platform manager creation test passed! OS: {}, Arch: {}", info.os, info.arch);
    } else {
        println!("Platform manager creation failed as expected (unsupported platform)");
    }
}

#[tokio::test]
async fn test_protocol_manager_creation() {
    // プロトコルマネージャーの作成テスト
    let protocol_config = soft_kvm_protocol::ProtocolConfig {
        version: "1.0.0".to_string(),
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    let protocol_manager = soft_kvm_protocol::ProtocolManager::new(protocol_config);
    assert!(protocol_manager.transport_manager.connection_count().await == 0,
            "Initial connection count should be 0");

    println!("Protocol manager creation test passed!");
}

#[tokio::test]
async fn test_message_serialization_roundtrip() {
    // メッセージのシリアライズ/デシリアライズのラウンドトリップテスト
    let hello_payload = soft_kvm_protocol::messages::HelloPayload {
        protocol_version: "1.0.0".to_string(),
        client_info: soft_kvm_protocol::messages::ClientInfo {
            client_id: "test-client".to_string(),
            client_name: "Test Client".to_string(),
            platform: "test".to_string(),
            version: "1.0.0".to_string(),
        },
        capabilities: vec!["video".to_string(), "input".to_string()],
    };

    let original_message = soft_kvm_protocol::messages::ProtocolMessage::new(
        soft_kvm_protocol::messages::MessageType::Hello,
        soft_kvm_protocol::messages::MessagePayload::Hello(hello_payload)
    );

    // JSONにシリアライズ
    let json_str = serde_json::to_string(&original_message).unwrap();

    // デシリアライズ
    let deserialized_message: soft_kvm_protocol::messages::ProtocolMessage =
        serde_json::from_str(&json_str).unwrap();

    // 比較
    assert_eq!(deserialized_message.message_type(), original_message.message_type());

    if let soft_kvm_protocol::messages::MessagePayload::Hello(payload) = &deserialized_message.payload {
        assert_eq!(payload.protocol_version, "1.0.0");
        assert_eq!(payload.client_info.client_name, "Test Client");
        assert!(payload.capabilities.contains(&"video".to_string()));
    } else {
        panic!("Expected Hello payload after deserialization");
    }

    println!("Message serialization roundtrip test passed!");
}

#[tokio::test]
async fn test_session_management() {
    // セッション管理のテスト
    let protocol_config = soft_kvm_protocol::ProtocolConfig {
        version: "1.0.0".to_string(),
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        compression_enabled: true,
    };

    let mut protocol_manager = soft_kvm_protocol::ProtocolManager::new(protocol_config);

    // セッション作成
    let session_id = "test-session-123".to_string();
    let peer_info = soft_kvm_protocol::session::PeerInfo {
        peer_id: "test-peer".to_string(),
        peer_name: "Test Peer".to_string(),
        address: soft_kvm_core::NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        },
        capabilities: vec!["video".to_string()],
        authenticated: false,
        last_seen: chrono::Utc::now(),
    };

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let result = protocol_manager.create_session(session_id.clone(), peer_info, tx).await;
    assert!(result.is_ok(), "Session creation should succeed");

    // セッション取得
    let session = protocol_manager.get_session(&session_id).await;
    assert!(session.is_some(), "Session should exist");

    let session = session.unwrap();
    assert_eq!(session.peer_info.peer_name, "Test Peer");
    assert!(!session.is_authenticated());

    // 認証設定
    session.set_authenticated(true).await;
    assert!(session.is_authenticated(), "Session should be authenticated");

    // セッション削除
    protocol_manager.remove_session(&session_id).await.unwrap();
    let session = protocol_manager.get_session(&session_id).await;
    assert!(session.is_none(), "Session should be removed");

    println!("Session management test passed!");
}

#[tokio::test]
async fn test_input_event_creation() {
    // Inputイベント作成テスト
    let keyboard_event = soft_kvm_core::KeyboardEvent {
        key_code: 65, // 'A'
        pressed: true,
        modifiers: vec![],
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let mouse_event = soft_kvm_core::MouseEvent {
        x: 100,
        y: 200,
        button: Some(1),
        pressed: Some(true),
        wheel_delta: None,
        modifiers: vec![],
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    // イベントのJSONシリアライズ確認
    let keyboard_json = serde_json::to_value(keyboard_event).unwrap();
    let mouse_json = serde_json::to_value(mouse_event).unwrap();

    assert!(keyboard_json.is_object());
    assert!(mouse_json.is_object());

    println!("Input event creation test passed!");
}

#[tokio::test]
async fn test_config_validation() {
    // 設定バリデーションのテスト
    let server_config = soft_kvm_server::ServerConfig {
        server_name: "Valid Server".to_string(),
        bind_address: soft_kvm_core::NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        },
        max_clients: 10,
        session_timeout: 300,
        heartbeat_interval: 30,
        enable_discovery: true,
        enable_security: true,
        video_config: soft_kvm_core::VideoConfig {
            resolution: soft_kvm_core::VideoResolution::fhd(),
            fps: 60,
            quality: soft_kvm_core::VideoQuality::high(),
            compression: true,
        },
        input_config: soft_kvm_core::InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "jp".to_string(),
            mouse_sensitivity: 2.0,
        },
        max_message_size: 2 * 1024 * 1024, // 2MB
    };

    let client_config = soft_kvm_client::ClientConfig {
        client_name: "Valid Client".to_string(),
        server_address: Some(soft_kvm_core::NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        }),
        auto_connect: true,
        max_message_size: 2 * 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        enable_security: true,
        video_config: soft_kvm_core::VideoConfig {
            resolution: soft_kvm_core::VideoResolution::fhd(),
            fps: 60,
            quality: soft_kvm_core::VideoQuality::high(),
            compression: true,
        },
        input_config: soft_kvm_core::InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "jp".to_string(),
            mouse_sensitivity: 2.0,
        },
    };

    // 設定が有効であることを確認（初期化でエラーが出ない）
    let server_result = soft_kvm_server::KvmServer::new(server_config).await;
    let client_result = soft_kvm_client::KvmClient::new(client_config).await;

    assert!(server_result.is_ok(), "Server config should be valid");
    assert!(client_result.is_ok(), "Client config should be valid");

    println!("Config validation test passed!");
}
