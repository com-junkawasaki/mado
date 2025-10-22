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

//! Utility functions for Soft KVM

use chrono::{DateTime, Utc, Duration};
use crate::{NetworkAddress, ServiceId, Resolution, ServiceType, KvmResult};

/// Generate a new service ID
pub fn generate_service_id() -> ServiceId {
    ServiceId(uuid::Uuid::new_v4())
}

/// Parse network address from string
pub fn parse_address(address: &str) -> KvmResult<NetworkAddress> {
    let parts: Vec<&str> = address.split(':').collect();
    if parts.len() != 2 {
        return Err(crate::KvmError::GenericError("Invalid address format".to_string()));
    }

    let ip = parts[0].to_string();
    let port = parts[1].parse::<u16>().map_err(|_| {
        crate::KvmError::GenericError("Invalid port number".to_string())
    })?;

    Ok(NetworkAddress { ip, port })
}

/// Check if resolution is expired
pub fn is_resolution_expired(resolution: &Resolution, timeout_seconds: u64) -> bool {
    let now = Utc::now();
    let elapsed = now.signed_duration_since(resolution.last_seen);
    elapsed > Duration::seconds(timeout_seconds as i64)
}

/// Format duration for display
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Calculate average from vector of values
pub fn calculate_average(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Calculate 99th percentile from sorted vector
pub fn calculate_p99(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        let index = (values.len() as f64 * 0.99) as usize;
        let safe_index = index.min(values.len() - 1);
        values[safe_index]
    }
}

/// Validate video resolution string
pub fn validate_resolution(resolution: &str) -> bool {
    let parts: Vec<&str> = resolution.split('x').collect();
    if parts.len() != 2 {
        return false;
    }

    parts[0].parse::<u32>().is_ok() && parts[1].parse::<u32>().is_ok()
}

/// Get default video configuration
pub fn default_video_config() -> crate::VideoConfig {
    crate::VideoConfig {
        resolution: "1920x1080".to_string(),
        fps: 30,
        quality: "balanced".to_string(),
    }
}

/// Get default input configuration
pub fn default_input_config() -> crate::InputConfig {
    crate::InputConfig {
        keyboard_enabled: true,
        mouse_enabled: true,
    }
}

/// Get default network configuration
pub fn default_network_config() -> crate::NetworkConfig {
    crate::NetworkConfig {
        discovery_enabled: true,
        auto_connect: false,
    }
}
