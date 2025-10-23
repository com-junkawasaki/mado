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

//! Server configuration management

use crate::{ServerConfig, ServerResult};
use soft_kvm_core::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Server configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfigFile {
    pub server: ServerConfigData,
}

/// Server configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfigData {
    pub name: String,
    pub bind_address: String,
    pub port: u16,
    pub max_clients: usize,
    pub session_timeout: u64,
    pub heartbeat_interval: u64,
    pub enable_discovery: bool,
    pub enable_security: bool,
    pub video: VideoConfigData,
    pub input: InputConfigData,
}

/// Video configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfigData {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: String,
    pub compression: bool,
}

/// Input configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfigData {
    pub enable_keyboard: bool,
    pub enable_mouse: bool,
    pub keyboard_layout: String,
    pub mouse_sensitivity: f64,
}

impl Default for ServerConfigFile {
    fn default() -> Self {
        ServerConfigFile {
            server: ServerConfigData {
                name: "Soft KVM Server".to_string(),
                bind_address: "0.0.0.0".to_string(),
                port: 8080,
                max_clients: 5,
                session_timeout: 300,
                heartbeat_interval: 30,
                enable_discovery: true,
                enable_security: true,
                video: VideoConfigData {
                    width: 1920,
                    height: 1080,
                    fps: 30,
                    quality: "balanced".to_string(),
                    compression: true,
                },
                input: InputConfigData {
                    enable_keyboard: true,
                    enable_mouse: true,
                    keyboard_layout: "us".to_string(),
                    mouse_sensitivity: 1.0,
                },
            },
        }
    }
}

impl ServerConfigFile {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> ServerResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ServerConfigFile = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ServerResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to ServerConfig
    pub fn to_server_config(&self) -> ServerResult<ServerConfig> {
        let video_quality = match self.server.video.quality.as_str() {
            "low" => VideoQuality {
                resolution: "640x480".to_string(),
                fps: 15,
                bitrate: 1000,
            },
            "balanced" => VideoQuality::balanced(),
            "high" => VideoQuality {
                resolution: "1920x1080".to_string(),
                fps: 60,
                bitrate: 10000,
            },
            _ => return Err(crate::ServerError::Config("Invalid video quality".to_string())),
        };

        let video_config = VideoConfig {
            resolution: VideoResolution::Custom(self.server.video.width, self.server.video.height),
            fps: self.server.video.fps,
            quality: video_quality,
            compression: self.server.video.compression,
        };

        let input_config = InputConfig {
            enable_keyboard: self.server.input.enable_keyboard,
            enable_mouse: self.server.input.enable_mouse,
            keyboard_layout: self.server.input.keyboard_layout.clone(),
            mouse_sensitivity: self.server.input.mouse_sensitivity,
        };

        Ok(ServerConfig {
            server_name: self.server.name.clone(),
            bind_address: NetworkAddress {
                ip: self.server.bind_address.clone(),
                port: self.server.port,
            },
            max_clients: self.server.max_clients,
            session_timeout: self.server.session_timeout,
            heartbeat_interval: self.server.heartbeat_interval,
            enable_discovery: self.server.enable_discovery,
            enable_security: self.server.enable_security,
            video_config,
            input_config,
        })
    }
}

impl From<ServerConfig> for ServerConfigFile {
    fn from(config: ServerConfig) -> Self {
        let (width, height) = (config.video_config.resolution.width, config.video_config.resolution.height);

        let quality = if config.video_config.quality.bitrate <= 2000 {
            "low"
        } else if config.video_config.quality.bitrate >= 8000 {
            "high"
        } else {
            "balanced"
        };

        ServerConfigFile {
            server: ServerConfigData {
                name: config.server_name,
                bind_address: config.bind_address.ip,
                port: config.bind_address.port,
                max_clients: config.max_clients,
                session_timeout: config.session_timeout,
                heartbeat_interval: config.heartbeat_interval,
                enable_discovery: config.enable_discovery,
                enable_security: config.enable_security,
                video: VideoConfigData {
                    width,
                    height,
                    fps: config.video_config.fps,
                    quality: quality.to_string(),
                    compression: config.video_config.compression,
                },
                input: InputConfigData {
                    enable_keyboard: config.input_config.enable_keyboard,
                    enable_mouse: config.input_config.enable_mouse,
                    keyboard_layout: config.input_config.keyboard_layout,
                    mouse_sensitivity: config.input_config.mouse_sensitivity,
                },
            },
        }
    }
}
