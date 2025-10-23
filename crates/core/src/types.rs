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

//! Core types for Soft KVM

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Service identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub uuid::Uuid);

/// Network address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAddress {
    pub ip: String,
    pub port: u16,
}

impl NetworkAddress {
    pub fn localhost(port: u16) -> Self {
        NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port,
        }
    }
}

/// Video quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoQuality {
    pub resolution: String,
    pub fps: u32,
    pub bitrate: u32,
}

impl VideoQuality {
    pub fn balanced() -> Self {
        VideoQuality {
            resolution: "1920x1080".to_string(),
            fps: 30,
            bitrate: 5000, // 5 Mbps
        }
    }
}

/// Video resolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VideoResolution {
    pub width: u32,
    pub height: u32,
}

impl VideoResolution {
    pub fn fhd() -> Self {
        VideoResolution {
            width: 1920,
            height: 1080,
        }
    }
}

/// Service resolution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub service_type: ServiceType,
    pub address: NetworkAddress,
    pub quality: Option<VideoQuality>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Service types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Server,
    Client,
}

/// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_bytes: u64,
    pub active_connections: u32,
    pub video_latency_p99: f64,
    pub input_latency_p99: f64,
}

/// Input configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub enable_keyboard: bool,
    pub enable_mouse: bool,
    pub keyboard_layout: String,
    pub mouse_sensitivity: f64,
}

/// Video configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub resolution: VideoResolution,
    pub fps: u32,
    pub quality: VideoQuality,
    pub compression: bool,
}

/// Keyboard event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyboardEvent {
    KeyPress { key_code: u32, modifiers: u32 },
    KeyRelease { key_code: u32, modifiers: u32 },
}

/// Mouse event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    MouseMove { x: i32, y: i32, relative: bool },
    MouseButtonPress { button: MouseButton },
    MouseButtonRelease { button: MouseButton },
    MouseScroll { delta_x: i32, delta_y: i32 },
}

/// Mouse button types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
}

/// Video frame data
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: VideoFormat,
    pub timestamp: u64,
}

/// Video format types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoFormat {
    Rgba,
    Bgra,
    Yuv420,
    Jpeg,
    H264,
}

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub video: VideoConfig,
    pub input: InputConfig,
    pub network: NetworkConfig,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub discovery_enabled: bool,
    pub auto_connect: bool,
}
