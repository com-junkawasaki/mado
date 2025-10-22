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

//! Video capture utilities

use crate::VideoConfig;

/// Validate video configuration
pub fn validate_config(config: &VideoConfig) -> Result<(), String> {
    // Validate resolution
    let parts: Vec<&str> = config.resolution.split('x').collect();
    if parts.len() != 2 {
        return Err("Invalid resolution format. Expected 'WIDTHxHEIGHT'".to_string());
    }

    if parts[0].parse::<u32>().is_err() || parts[1].parse::<u32>().is_err() {
        return Err("Resolution must contain valid numbers".to_string());
    }

    // Validate FPS
    if config.fps == 0 || config.fps > 240 {
        return Err("FPS must be between 1 and 240".to_string());
    }

    // Validate quality
    match config.quality.as_str() {
        "low" | "balanced" | "high" => {}
        _ => return Err("Quality must be 'low', 'balanced', or 'high'".to_string()),
    }

    Ok(())
}

/// Calculate frame interval in milliseconds
pub fn calculate_frame_interval(fps: u32) -> u64 {
    if fps == 0 {
        1000 // Default to 1 FPS
    } else {
        1000 / fps as u64
    }
}