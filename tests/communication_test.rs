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

//! Basic communication flow tests

use soft_kvm_core::*;
use soft_kvm_protocol::{messages::*, *};
use soft_kvm_server::{KvmServer, ServerConfig, handler::ServerMessageHandler};
use soft_kvm_client::{KvmClient, ClientConfig, handler::ClientMessageHandler};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[tokio::test]
async fn test_hello_message_flow() {
    // Test Hello message creation and parsing
    let hello_payload = HelloPayload {
        protocol_version: "1.0.0".to_string(),
        client_info: ClientInfo {
            client_id: "test-client".to_string(),
            client_name: "Test Client".to_string(),
            platform: "test".to_string(),
            version: "1.0.0".to_string(),
        },
        capabilities: vec!["video".to_string(), "input".to_string()],
    };

    let message = ProtocolMessage::new(MessageType::Hello, MessagePayload::Hello(hello_payload.clone()));

    // Verify message structure
    assert_eq!(message.message_type(), MessageType::Hello);

    if let MessagePayload::Hello(payload) = &message.payload {
        assert_eq!(payload.protocol_version, "1.0.0");
        assert_eq!(payload.client_info.client_name, "Test Client");
        assert!(payload.capabilities.contains(&"video".to_string()));
        assert!(payload.capabilities.contains(&"input".to_string()));
    } else {
        panic!("Expected Hello payload");
    }

    println!("Hello message flow test passed!");
}

#[tokio::test]
async fn test_auth_message_flow() {
    // Test AuthRequest and AuthResponse messages
    let auth_request = AuthRequestPayload {
        auth_method: "password".to_string(),
        credentials: serde_json::json!({
            "username": "testuser",
            "password_hash": "dummy_hash"
        }),
    };

    let request_message = ProtocolMessage::new(
        MessageType::AuthRequest,
        MessagePayload::AuthRequest(auth_request.clone())
    );

    // Verify request message
    assert_eq!(request_message.message_type(), MessageType::AuthRequest);

    if let MessagePayload::AuthRequest(payload) = &request_message.payload {
        assert_eq!(payload.auth_method, "password");
        assert!(payload.credentials.is_object());
    } else {
        panic!("Expected AuthRequest payload");
    }

    // Test AuthResponse
    let auth_response = AuthResponsePayload {
        success: true,
        session_token: Some("test-token".to_string()),
        error_message: None,
    };

    let response_message = ProtocolMessage::new(
        MessageType::AuthResponse,
        MessagePayload::AuthResponse(auth_response.clone())
    );

    assert_eq!(response_message.message_type(), MessageType::AuthResponse);

    if let MessagePayload::AuthResponse(payload) = &response_message.payload {
        assert!(payload.success);
        assert_eq!(payload.session_token, Some("test-token".to_string()));
        assert!(payload.error_message.is_none());
    } else {
        panic!("Expected AuthResponse payload");
    }

    println!("Auth message flow test passed!");
}

#[tokio::test]
async fn test_heartbeat_message_flow() {
    // Test Heartbeat message
    let heartbeat_payload = HeartbeatPayload {
        sequence_number: 42,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let heartbeat_message = ProtocolMessage::new(
        MessageType::Heartbeat,
        MessagePayload::Heartbeat(heartbeat_payload.clone())
    );

    assert_eq!(heartbeat_message.message_type(), MessageType::Heartbeat);

    if let MessagePayload::Heartbeat(payload) = &heartbeat_message.payload {
        assert_eq!(payload.sequence_number, 42);
        assert!(payload.timestamp > 0);
    } else {
        panic!("Expected Heartbeat payload");
    }

    // Test Pong response
    let pong_message = ProtocolMessage::new(
        MessageType::Pong,
        MessagePayload::Pong
    );

    assert_eq!(pong_message.message_type(), MessageType::Pong);

    println!("Heartbeat message flow test passed!");
}

#[tokio::test]
async fn test_input_event_message_flow() {
    // Test InputEvent message with keyboard event
    let keyboard_event = KeyboardEvent {
        key_code: 65, // 'A' key
        pressed: true,
        modifiers: vec![],
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let input_payload = InputEventPayload {
        event_type: "keyboard".to_string(),
        data: serde_json::to_value(keyboard_event.clone()).unwrap(),
    };

    let input_message = ProtocolMessage::new(
        MessageType::InputEvent,
        MessagePayload::InputEvent(input_payload.clone())
    );

    assert_eq!(input_message.message_type(), MessageType::InputEvent);

    if let MessagePayload::InputEvent(payload) = &input_message.payload {
        assert_eq!(payload.event_type, "keyboard");

        // Deserialize the keyboard event
        let deserialized: KeyboardEvent = serde_json::from_value(payload.data.clone()).unwrap();
        assert_eq!(deserialized.key_code, keyboard_event.key_code);
        assert_eq!(deserialized.pressed, keyboard_event.pressed);
    } else {
        panic!("Expected InputEvent payload");
    }

    // Test with mouse event
    let mouse_event = MouseEvent {
        x: 100,
        y: 200,
        button: Some(1),
        pressed: Some(true),
        wheel_delta: None,
        modifiers: vec![],
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let mouse_payload = InputEventPayload {
        event_type: "mouse".to_string(),
        data: serde_json::to_value(mouse_event.clone()).unwrap(),
    };

    let mouse_message = ProtocolMessage::new(
        MessageType::InputEvent,
        MessagePayload::InputEvent(mouse_payload.clone())
    );

    if let MessagePayload::InputEvent(payload) = &mouse_message.payload {
        assert_eq!(payload.event_type, "mouse");

        let deserialized: MouseEvent = serde_json::from_value(payload.data.clone()).unwrap();
        assert_eq!(deserialized.x, mouse_event.x);
        assert_eq!(deserialized.y, mouse_event.y);
        assert_eq!(deserialized.button, mouse_event.button);
    } else {
        panic!("Expected InputEvent payload");
    }

    println!("Input event message flow test passed!");
}

#[tokio::test]
async fn test_goodbye_message_flow() {
    // Test Goodbye message
    let goodbye_payload = GoodbyePayload {
        reason: "Client disconnecting".to_string(),
        code: 0,
    };

    let goodbye_message = ProtocolMessage::new(
        MessageType::Goodbye,
        MessagePayload::Goodbye(goodbye_payload.clone())
    );

    assert_eq!(goodbye_message.message_type(), MessageType::Goodbye);

    if let MessagePayload::Goodbye(payload) = &goodbye_message.payload {
        assert_eq!(payload.reason, "Client disconnecting");
        assert_eq!(payload.code, 0);
    } else {
        panic!("Expected Goodbye payload");
    }

    println!("Goodbye message flow test passed!");
}

#[tokio::test]
async fn test_session_id_handling() {
    // Test session ID handling in messages
    let message = ProtocolMessage::new(
        MessageType::Hello,
        MessagePayload::Hello(HelloPayload {
            protocol_version: "1.0.0".to_string(),
            client_info: ClientInfo {
                client_id: "test-client".to_string(),
                client_name: "Test Client".to_string(),
                platform: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: vec![],
        })
    );

    // Initially no session ID
    assert!(message.session_id().is_none());

    // Add session ID
    let message_with_session = message.with_session("test-session-123".to_string());

    assert_eq!(message_with_session.session_id(), Some(&"test-session-123".to_string()));

    println!("Session ID handling test passed!");
}

#[tokio::test]
async fn test_server_client_config_creation() {
    // Test server config creation
    let server_config = ServerConfig::default();
    assert!(!server_config.server_name.is_empty());
    assert!(!server_config.bind_address.ip.is_empty());
    assert!(server_config.bind_address.port > 0);

    // Test client config creation
    let client_config = ClientConfig::default();
    assert!(!client_config.client_name.is_empty());
    assert!(!client_config.server_address.as_ref().unwrap().ip.is_empty());
    assert!(client_config.server_address.as_ref().unwrap().port > 0);

    println!("Server/Client config creation test passed!");
}

#[tokio::test]
async fn test_message_serialization() {
    // Test JSON serialization/deserialization of messages
    let hello_payload = HelloPayload {
        protocol_version: "1.0.0".to_string(),
        client_info: ClientInfo {
            client_id: "test-client".to_string(),
            client_name: "Test Client".to_string(),
            platform: "test".to_string(),
            version: "1.0.0".to_string(),
        },
        capabilities: vec!["video".to_string()],
    };

    let original_message = ProtocolMessage::new(
        MessageType::Hello,
        MessagePayload::Hello(hello_payload)
    );

    // Serialize to JSON
    let json_str = serde_json::to_string(&original_message).unwrap();

    // Deserialize back
    let deserialized_message: ProtocolMessage = serde_json::from_str(&json_str).unwrap();

    // Verify they match
    assert_eq!(deserialized_message.message_type(), original_message.message_type());

    if let MessagePayload::Hello(payload) = &deserialized_message.payload {
        assert_eq!(payload.protocol_version, "1.0.0");
        assert_eq!(payload.client_info.client_name, "Test Client");
    } else {
        panic!("Expected Hello payload after deserialization");
    }

    println!("Message serialization test passed!");
}
