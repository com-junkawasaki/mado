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

//! Tauri plugin integration tests

use std::sync::Arc;
use tokio::sync::RwLock;

// Test that plugins can be imported and initialized
#[tokio::test]
async fn test_plugin_imports() {
    // Test input plugin initialization
    let input_plugin = soft_kvm_plugin_input::init();
    println!("Input plugin initialized successfully");

    // Test service plugin initialization
    let service_plugin = soft_kvm_plugin_service::init();
    println!("Service plugin initialized successfully");

    // Test security plugin initialization
    let security_plugin = soft_kvm_plugin_security::init();
    println!("Security plugin initialized successfully");

    println!("All active plugins initialized successfully!");
}

// Test basic plugin state management
#[tokio::test]
async fn test_plugin_states() {
    // Test input plugin state
    let input_state = Arc::new(RwLock::new(soft_kvm_plugin_input::InputCaptureState::default()));
    println!("Input plugin state created successfully");

    // Test service plugin state
    let service_state = Arc::new(RwLock::new(soft_kvm_plugin_service::ServiceState::default()));
    println!("Service plugin state created successfully");

    // Test security plugin state
    let security_state = Arc::new(RwLock::new(soft_kvm_plugin_security::SecurityState::default()));
    println!("Security plugin state created successfully");

    println!("All plugin states created successfully!");
}

// Test input plugin commands (without Tauri runtime)
#[tokio::test]
async fn test_input_plugin_functionality() {
    use soft_kvm_plugin_input::*;

    // Create a mock state
    let state = Arc::new(RwLock::new(InputCaptureState::default()));

    // Test get_input_status command
    let status_result = get_input_status(tauri::State::from(&state)).await;
    assert!(status_result.is_ok(), "get_input_status should succeed");

    let status = status_result.unwrap();
    assert_eq!(status["is_capturing"], false, "Should not be capturing initially");
    assert!(status["config"].is_null(), "Config should be null initially");

    println!("Input plugin status query works!");
}

// Test service plugin commands (without Tauri runtime)
#[tokio::test]
async fn test_service_plugin_functionality() {
    use soft_kvm_plugin_service::*;

    // Create a mock state
    let state = Arc::new(RwLock::new(ServiceState::default()));

    // Test get_service_status command
    let status_result = get_service_status(tauri::State::from(&state)).await;
    assert!(status_result.is_ok(), "get_service_status should succeed");

    println!("Service plugin status query works!");
}

// Test security plugin commands (without Tauri runtime)
#[tokio::test]
async fn test_security_plugin_functionality() {
    use soft_kvm_plugin_security::*;

    // Create a mock state
    let state = Arc::new(RwLock::new(SecurityState::default()));

    // Test get_security_status command
    let status_result = get_security_status(tauri::State::from(&state)).await;
    assert!(status_result.is_ok(), "get_security_status should succeed");

    println!("Security plugin status query works!");
}

#[tokio::test]
async fn test_plugin_initialization_sequence() {
    // Test the order of plugin initialization as done in main.rs

    // 1. Input plugin
    let _input_plugin = soft_kvm_plugin_input::init();

    // 2. Service plugin
    let _service_plugin = soft_kvm_plugin_service::init();

    // 3. Security plugin
    let _security_plugin = soft_kvm_plugin_security::init();

    // Protocol and discovery plugins are commented out in main.rs
    // let _protocol_plugin = soft_kvm_plugin_protocol::init();
    // let _discovery_plugin = soft_kvm_plugin_discovery::init();

    println!("Plugin initialization sequence test passed!");
}
