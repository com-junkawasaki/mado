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

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyboardEvent {
    pub key_code: u32,
    pub pressed: bool,
    pub modifiers: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub button: Option<u32>,
    pub pressed: Option<bool>,
    pub wheel_delta: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputConfig {
    pub keyboard_enabled: bool,
    pub mouse_enabled: bool,
}

/// Start input capture
#[tauri::command]
async fn start_input_capture(config: InputConfig) -> Result<String, String> {
    println!("Starting input capture with config: {:?}", config);
    Ok("Input capture started".to_string())
}

/// Stop input capture
#[tauri::command]
async fn stop_input_capture() -> Result<String, String> {
    println!("Stopping input capture");
    Ok("Input capture stopped".to_string())
}

/// Send keyboard event
#[tauri::command]
async fn send_keyboard_event(event: KeyboardEvent) -> Result<String, String> {
    println!("Sending keyboard event: {:?}", event);
    Ok("Keyboard event sent".to_string())
}

/// Send mouse event
#[tauri::command]
async fn send_mouse_event(event: MouseEvent) -> Result<String, String> {
    println!("Sending mouse event: {:?}", event);
    Ok("Mouse event sent".to_string())
}

/// Initialize the input plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-input")
        .invoke_handler(tauri::generate_handler![
            start_input_capture,
            stop_input_capture,
            send_keyboard_event,
            send_mouse_event,
        ])
        .build()
}
