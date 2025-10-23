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

//! Integration tests for Soft KVM

use soft_kvm_core::*;
use soft_kvm_protocol::*;
use soft_kvm_platform::*;
use soft_kvm_discovery::*;

#[tokio::test]
async fn test_basic_imports() {
    // Test that we can create basic types
    let network_addr = NetworkAddress::localhost(8080);
    assert_eq!(network_addr.ip, "127.0.0.1");
    assert_eq!(network_addr.port, 8080);

    let config = TransportConfig::default();
    assert_eq!(config.max_connections, 100);

    // Test protocol version
    assert_eq!(PROTOCOL_VERSION, "1.0.0");

    println!("Basic imports test passed!");
}

#[tokio::test]
async fn test_protocol_message_creation() {
    // Test basic message creation
    let payload = MessagePayload::Hello(HelloPayload {
        protocol_version: "1.0.0".to_string(),
        client_info: ClientInfo {
            client_id: "test-client".to_string(),
            client_name: "Test Client".to_string(),
            platform: "test".to_string(),
            version: "1.0.0".to_string(),
        },
        capabilities: vec!["video".to_string(), "input".to_string()],
    });

    let message = ProtocolMessage::new(MessageType::Hello, payload);

    // Check that message was created
    assert_eq!(message.message_type(), MessageType::Hello);
    assert!(message.session_id().is_none()); // No session ID set initially

    println!("Protocol message creation test passed!");
}

#[tokio::test]
async fn test_transport_factory() {
    // Test WebSocket factory creation
    let config = TransportConfig::default();
    let factory = WebSocketFactory::new(config);

    // Test listener creation (should not fail)
    let addr = "127.0.0.1:0".parse().unwrap();
    let listener_result = factory.create_listener(addr, TransportConfig::default()).await;
    assert!(listener_result.is_ok(), "Listener creation should succeed");

    println!("Transport factory test passed!");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_platform_linux() {
    // Test platform manager creation on Linux
    let platform_result = PlatformManager::new();
    assert!(platform_result.is_ok(), "Platform manager creation should succeed on Linux");

    println!("Platform Linux test passed!");
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_platform_macos() {
    // Test platform manager creation on macOS
    let platform_result = PlatformManager::new();
    assert!(platform_result.is_ok(), "Platform manager creation should succeed on macOS");

    println!("Platform macOS test passed!");
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_platform_windows() {
    // Test platform manager creation on Windows
    let platform_result = PlatformManager::new();
    assert!(platform_result.is_ok(), "Platform manager creation should succeed on Windows");

    println!("Platform Windows test passed!");
}
