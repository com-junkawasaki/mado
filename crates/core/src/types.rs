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

/// Video quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoQuality {
    pub resolution: String,
    pub fps: u32,
    pub bitrate: u32,
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

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub video: VideoConfig,
    pub input: InputConfig,
    pub network: NetworkConfig,
}

/// Video configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub resolution: String,
    pub fps: u32,
    pub quality: String,
}

/// Input configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub keyboard_enabled: bool,
    pub mouse_enabled: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub discovery_enabled: bool,
    pub auto_connect: bool,
}
