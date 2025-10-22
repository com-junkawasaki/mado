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

//! # Soft KVM Video Plugin
//!
//! Tauri plugin for video capture and encoding

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

mod capture;
mod platform;
mod encoder;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoConfig {
    pub resolution: String,
    pub fps: u32,
    pub quality: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub format: String, // "RGBA", "BGRA", etc.
}

#[derive(Debug)]
struct VideoCaptureState {
    is_capturing: bool,
    config: Option<VideoConfig>,
    capture_task: Option<tokio::task::JoinHandle<()>>,
}

impl Default for VideoCaptureState {
    fn default() -> Self {
        VideoCaptureState {
            is_capturing: false,
            config: None,
            capture_task: None,
        }
    }
}

/// Start video capture
#[tauri::command]
async fn start_capture(
    config: VideoConfig,
    state: tauri::State<'_, Arc<RwLock<VideoCaptureState>>>,
) -> Result<String, String> {
    let mut capture_state = state.write().await;

    if capture_state.is_capturing {
        return Err("Video capture is already running".to_string());
    }

    println!("Starting video capture with config: {:?}", config);

    // Initialize platform-specific capture
    match platform::initialize_capture(&config) {
        Ok(_) => {
            capture_state.is_capturing = true;
            capture_state.config = Some(config.clone());

            // Start capture task
            let state_clone = Arc::clone(&state);
            let task = tokio::spawn(async move {
                capture_loop(state_clone, config).await;
            });

            capture_state.capture_task = Some(task);

            Ok("Video capture started".to_string())
        }
        Err(e) => Err(format!("Failed to initialize video capture: {}", e)),
    }
}

/// Stop video capture
#[tauri::command]
async fn stop_capture(state: tauri::State<'_, Arc<RwLock<VideoCaptureState>>>) -> Result<String, String> {
    let mut capture_state = state.write().await;

    if !capture_state.is_capturing {
        return Ok("Video capture is not running".to_string());
    }

    println!("Stopping video capture");

    // Stop capture task
    if let Some(task) = capture_state.capture_task.take() {
        task.abort();
    }

    // Cleanup platform-specific capture
    platform::cleanup_capture();

    capture_state.is_capturing = false;
    capture_state.config = None;

    Ok("Video capture stopped".to_string())
}

/// Get current video frame
#[tauri::command]
async fn get_video_frame(state: tauri::State<'_, Arc<RwLock<VideoCaptureState>>>) -> Result<VideoFrame, String> {
    let capture_state = state.read().await;

    if !capture_state.is_capturing {
        return Err("Video capture is not running".to_string());
    }

    // Get frame from platform-specific implementation
    match platform::capture_frame() {
        Ok(frame) => Ok(frame),
        Err(e) => Err(format!("Failed to capture frame: {}", e)),
    }
}

/// Get capture status
#[tauri::command]
async fn get_capture_status(state: tauri::State<'_, Arc<RwLock<VideoCaptureState>>>) -> Result<serde_json::Value, String> {
    let capture_state = state.read().await;

    let status = serde_json::json!({
        "is_capturing": capture_state.is_capturing,
        "config": capture_state.config,
    });

    Ok(status)
}

async fn capture_loop(state: Arc<RwLock<VideoCaptureState>>, config: VideoConfig) {
    let frame_interval = std::time::Duration::from_millis(1000 / config.fps as u64);

    loop {
        let start_time = std::time::Instant::now();

        // Capture frame (this would be implemented in platform module)
        // For now, just sleep to maintain frame rate
        tokio::time::sleep(frame_interval.saturating_sub(start_time.elapsed())).await;

        // Check if we should continue
        let should_continue = {
            let capture_state = state.read().await;
            capture_state.is_capturing
        };

        if !should_continue {
            break;
        }
    }
}

/// Initialize the video plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-video")
        .invoke_handler(tauri::generate_handler![
            start_capture,
            stop_capture,
            get_video_frame,
            get_capture_status,
        ])
        .setup(|_app| {
            // Initialize capture state
            let state = Arc::new(RwLock::new(VideoCaptureState::default()));
            _app.manage(state);
            Ok(())
        })
        .build()
}
