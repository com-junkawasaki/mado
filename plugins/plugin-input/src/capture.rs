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

//! Input capture utilities

use crate::InputConfig;

/// Validate input configuration
pub fn validate_config(config: &InputConfig) -> Result<(), String> {
    // Basic validation - ensure at least one input type is enabled
    if !config.keyboard_enabled && !config.mouse_enabled {
        return Err("At least keyboard or mouse input must be enabled".to_string());
    }

    Ok(())
}

/// Convert platform-specific key codes to standard codes
pub fn normalize_key_code(platform_key_code: u32, platform: &str) -> u32 {
    // TODO: Implement key code normalization across platforms
    // For now, return as-is
    match platform {
        "macos" => platform_key_code,
        "linux" => platform_key_code,
        "windows" => platform_key_code,
        _ => platform_key_code,
    }
}

/// Convert platform-specific mouse button codes to standard codes
pub fn normalize_mouse_button(platform_button: u32, platform: &str) -> u32 {
    // Standard mouse button mapping
    // 0: Left, 1: Right, 2: Middle, 3+: Extra buttons
    match platform {
        "macos" => platform_button,
        "linux" => platform_button,
        "windows" => platform_button,
        _ => platform_button,
    }
}