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

//! Input injection utilities

use crate::{KeyboardEvent, MouseEvent};

/// Validate keyboard event before injection
pub fn validate_keyboard_event(event: &KeyboardEvent) -> Result<(), String> {
    // Basic validation
    if event.key_code > 255 {
        return Err("Invalid key code".to_string());
    }

    Ok(())
}

/// Validate mouse event before injection
pub fn validate_mouse_event(event: &MouseEvent) -> Result<(), String> {
    // Basic validation
    if let Some(button) = event.button {
        if button > 15 {
            return Err("Invalid mouse button".to_string());
        }
    }

    Ok(())
}

/// Create synthetic keyboard event for testing
pub fn create_test_keyboard_event(key_code: u32, pressed: bool) -> KeyboardEvent {
    KeyboardEvent {
        key_code,
        pressed,
        modifiers: 0,
    }
}

/// Create synthetic mouse event for testing
pub fn create_test_mouse_event(x: i32, y: i32, button: Option<u32>, pressed: Option<bool>) -> MouseEvent {
    MouseEvent {
        x,
        y,
        button,
        pressed,
        wheel_delta: None,
    }
}

/// Calculate relative mouse movement
pub fn calculate_relative_movement(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> (i32, i32) {
    (to_x - from_x, to_y - from_y)
}

/// Apply acceleration to mouse movement (for smoother remote control)
pub fn apply_mouse_acceleration(delta_x: i32, delta_y: i32, acceleration: f32) -> (i32, i32) {
    let accel_x = (delta_x as f32 * acceleration) as i32;
    let accel_y = (delta_y as f32 * acceleration) as i32;
    (accel_x, accel_y)
}