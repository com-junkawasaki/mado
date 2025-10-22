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

//! # Soft KVM Input Plugin
//!
//! Tauri plugin for keyboard and mouse input handling

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

mod capture;
mod injection;
mod platform;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyboardEvent {
    pub key_code: u32,
    pub pressed: bool,
    pub modifiers: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub button: Option<u32>,
    pub pressed: Option<bool>,
    pub wheel_delta: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InputConfig {
    pub keyboard_enabled: bool,
    pub mouse_enabled: bool,
    pub toggle_hotkey: Option<HotkeyConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HotkeyConfig {
    pub modifiers: Vec<String>,
    pub key: String,
}

#[derive(Debug)]
struct InputCaptureState {
    is_capturing: bool,
    config: Option<InputConfig>,
    capture_task: Option<tokio::task::JoinHandle<()>>,
    event_sender: Option<tokio::sync::mpsc::Sender<InputEvent>>,
}

impl Default for InputCaptureState {
    fn default() -> Self {
        InputCaptureState {
            is_capturing: false,
            config: None,
            capture_task: None,
            event_sender: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
}

/// Start input capture
#[tauri::command]
async fn start_input_capture(
    config: InputConfig,
    state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>,
) -> Result<String, String> {
    let mut capture_state = state.write().await;

    if capture_state.is_capturing {
        return Err("Input capture is already running".to_string());
    }

    println!("Starting input capture with config: {:?}", config);

    // Create event channel
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    // Initialize platform-specific capture
    match platform::initialize_input_capture(&config) {
        Ok(_) => {
            capture_state.is_capturing = true;
            capture_state.config = Some(config.clone());
            capture_state.event_sender = Some(tx.clone());

            // Start capture task
            let state_clone = Arc::clone(&state);
            let task = tokio::spawn(async move {
                input_capture_loop(state_clone, rx, config).await;
            });

            capture_state.capture_task = Some(task);

            Ok("Input capture started".to_string())
        }
        Err(e) => Err(format!("Failed to initialize input capture: {}", e)),
    }
}

/// Stop input capture
#[tauri::command]
async fn stop_input_capture(state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>) -> Result<String, String> {
    let mut capture_state = state.write().await;

    if !capture_state.is_capturing {
        return Ok("Input capture is not running".to_string());
    }

    println!("Stopping input capture");

    // Stop capture task
    if let Some(task) = capture_state.capture_task.take() {
        task.abort();
    }

    // Close event channel
    capture_state.event_sender = None;

    // Cleanup platform-specific capture
    platform::cleanup_input_capture();

    capture_state.is_capturing = false;
    capture_state.config = None;

    Ok("Input capture stopped".to_string())
}

/// Send keyboard event to remote system
#[tauri::command]
async fn send_keyboard_event(
    mut event: KeyboardEvent,
    state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>,
) -> Result<String, String> {
    let capture_state = state.read().await;

    if !capture_state.is_capturing {
        return Err("Input capture is not running".to_string());
    }

    // Validate event
    if let Err(e) = injection::validate_keyboard_event(&event) {
        return Err(format!("Invalid keyboard event: {}", e));
    }

    // Normalize key code (assuming "universal" platform for now)
    event.key_code = capture::normalize_key_code(event.key_code, "universal");

    println!("Sending keyboard event: {:?}", event);

    // Send event to remote system via platform-specific injection
    match platform::inject_keyboard_event(&event) {
        Ok(_) => Ok("Keyboard event sent".to_string()),
        Err(e) => Err(format!("Failed to send keyboard event: {}", e)),
    }
}

/// Send mouse event to remote system
#[tauri::command]
async fn send_mouse_event(
    mut event: MouseEvent,
    state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>,
) -> Result<String, String> {
    let capture_state = state.read().await;

    if !capture_state.is_capturing {
        return Err("Input capture is not running".to_string());
    }

    // Validate event
    if let Err(e) = injection::validate_mouse_event(&event) {
        return Err(format!("Invalid mouse event: {}", e));
    }

    // Normalize mouse button if present
    if let Some(button) = event.button {
        event.button = Some(capture::normalize_mouse_button(button, "universal"));
    }

    println!("Sending mouse event: {:?}", event);

    // Send event to remote system via platform-specific injection
    match platform::inject_mouse_event(&event) {
        Ok(_) => Ok("Mouse event sent".to_string()),
        Err(e) => Err(format!("Failed to send mouse event: {}", e)),
    }
}

/// Get input capture status
#[tauri::command]
async fn get_input_status(state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>) -> Result<serde_json::Value, String> {
    let capture_state = state.read().await;

    let status = serde_json::json!({
        "is_capturing": capture_state.is_capturing,
        "config": capture_state.config,
    });

    Ok(status)
}

/// Toggle input capture using hotkey
#[tauri::command]
async fn toggle_input_capture(
    hotkey: HotkeyConfig,
    state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>,
) -> Result<String, String> {
    let mut capture_state = state.write().await;

    // Validate hotkey
    if hotkey.key.is_empty() {
        return Err("Hotkey key cannot be empty".to_string());
    }

    println!("Toggle hotkey pressed: {:?}", hotkey);

    if capture_state.is_capturing {
        // Stop capture
        if let Some(task) = capture_state.capture_task.take() {
            task.abort();
        }
        capture_state.event_sender = None;
        platform::cleanup_input_capture();
        capture_state.is_capturing = false;
        capture_state.config = None;

        Ok("Input capture toggled OFF".to_string())
    } else {
        // Start capture with default config
        let config = InputConfig {
            keyboard_enabled: true,
            mouse_enabled: true,
            toggle_hotkey: Some(hotkey),
        };

        // Create event channel
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Initialize platform-specific capture
        match platform::initialize_input_capture(&config) {
            Ok(_) => {
                capture_state.is_capturing = true;
                capture_state.config = Some(config.clone());
                capture_state.event_sender = Some(tx.clone());

                // Start capture task
                let state_clone = Arc::clone(&state);
                let task = tokio::spawn(async move {
                    input_capture_loop(state_clone, rx, config).await;
                });

                capture_state.capture_task = Some(task);

                Ok("Input capture toggled ON".to_string())
            }
            Err(e) => Err(format!("Failed to initialize input capture: {}", e)),
        }
    }
}

/// Set toggle hotkey
#[tauri::command]
async fn set_toggle_hotkey(
    hotkey: Option<HotkeyConfig>,
    state: tauri::State<'_, Arc<RwLock<InputCaptureState>>>,
) -> Result<String, String> {
    let mut capture_state = state.write().await;

    if let Some(ref mut config) = capture_state.config {
        config.toggle_hotkey = hotkey.clone();
    }

    println!("Toggle hotkey set: {:?}", hotkey);
    Ok("Toggle hotkey updated".to_string())
}

async fn input_capture_loop(
    state: Arc<RwLock<InputCaptureState>>,
    mut rx: tokio::sync::mpsc::Receiver<InputEvent>,
    config: InputConfig,
) {
    println!("Input capture loop started");

    loop {
        tokio::select! {
            // Receive input events from platform-specific capture
            event = rx.recv() => {
                match event {
                    Some(InputEvent::Keyboard(keyboard_event)) => {
                        // Process keyboard event (send to remote, record locally, etc.)
                        println!("Captured keyboard event: {:?}", keyboard_event);
                    }
                    Some(InputEvent::Mouse(mouse_event)) => {
                        // Process mouse event
                        println!("Captured mouse event: {:?}", mouse_event);
                    }
                    None => {
                        // Channel closed, exit loop
                        break;
                    }
                }
            }
            // Check if we should continue
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let should_continue = {
                    let capture_state = state.read().await;
                    capture_state.is_capturing
                };

                if !should_continue {
                    break;
                }
            }
        }
    }

    println!("Input capture loop ended");
}

/// Initialize the input plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-input")
        .invoke_handler(tauri::generate_handler![
            start_input_capture,
            stop_input_capture,
            send_keyboard_event,
            send_mouse_event,
            get_input_status,
            toggle_input_capture,
            set_toggle_hotkey,
        ])
        .setup(|_app| {
            // Initialize capture state
            let state = Arc::new(RwLock::new(InputCaptureState::default()));
            _app.manage(state);
            Ok(())
        })
        .build()
}
