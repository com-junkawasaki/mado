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

//! Client configuration management

use crate::{ClientConfig, ClientResult};
use soft_kvm_core::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Client configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfigFile {
    pub client: ClientConfigData,
}

/// Client configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfigData {
    pub name: String,
    pub server_address: String,
    pub server_port: u16,
    pub auto_connect: bool,
    pub reconnect_attempts: u32,
    pub reconnect_delay: u64,
    pub session_timeout: u64,
    pub heartbeat_interval: u64,
    pub enable_discovery: bool,
    pub enable_security: bool,
}

impl Default for ClientConfigFile {
    fn default() -> Self {
        ClientConfigFile {
            client: ClientConfigData {
                name: "Soft KVM Client".to_string(),
                server_address: "127.0.0.1".to_string(),
                server_port: 8080,
                auto_connect: false,
                reconnect_attempts: 3,
                reconnect_delay: 5,
                session_timeout: 300,
                heartbeat_interval: 30,
                enable_discovery: true,
                enable_security: true,
            },
        }
    }
}

impl ClientConfigFile {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> ClientResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ClientConfigFile = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ClientResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to ClientConfig
    pub fn to_client_config(&self) -> ClientResult<ClientConfig> {
        Ok(ClientConfig {
            client_name: self.client.name.clone(),
            server_address: NetworkAddress {
                ip: self.client.server_address.clone(),
                port: self.client.server_port,
            },
            auto_connect: self.client.auto_connect,
            reconnect_attempts: self.client.reconnect_attempts,
            reconnect_delay: self.client.reconnect_delay,
            session_timeout: self.client.session_timeout,
            heartbeat_interval: self.client.heartbeat_interval,
            enable_discovery: self.client.enable_discovery,
            enable_security: self.client.enable_security,
        })
    }
}

impl From<ClientConfig> for ClientConfigFile {
    fn from(config: ClientConfig) -> Self {
        ClientConfigFile {
            client: ClientConfigData {
                name: config.client_name,
                server_address: config.server_address.ip,
                server_port: config.server_address.port,
                auto_connect: config.auto_connect,
                reconnect_attempts: config.reconnect_attempts,
                reconnect_delay: config.reconnect_delay,
                session_timeout: config.session_timeout,
                heartbeat_interval: config.heartbeat_interval,
                enable_discovery: config.enable_discovery,
                enable_security: config.enable_security,
            },
        }
    }
}
