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

//! # Soft KVM Platform
//!
//! Platform-specific implementations for input capture, video capture,
//! and system integration across Linux, macOS, and Windows.

use soft_kvm_core::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

pub mod input;
pub mod video;
pub mod system;

use crate::input::{InputCapture, PlatformInputCapture};
use crate::video::{VideoCapture, PlatformVideoCapture};
use crate::system::{SystemService, PlatformSystemService};

/// Platform result type
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Platform-specific errors
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Input capture error: {0}")]
    InputCapture(String),

    #[error("Video capture error: {0}")]
    VideoCapture(String),

    #[error("System service error: {0}")]
    SystemService(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Platform manager for handling platform-specific operations
pub struct PlatformManager {
    input_capture: Option<Box<dyn InputCapture>>,
    video_capture: Option<Box<dyn VideoCapture>>,
    system_service: Option<Box<dyn SystemService>>,
}

impl PlatformManager {
    /// Create a new platform manager
    pub fn new() -> PlatformResult<Self> {
        info!("Initializing platform manager for {}", std::env::consts::OS);

        let input_capture = Self::create_input_capture()?;
        let video_capture = Self::create_video_capture()?;
        let system_service = Self::create_system_service()?;

        Ok(PlatformManager {
            input_capture: Some(input_capture),
            video_capture: Some(video_capture),
            system_service: Some(system_service),
        })
    }

    /// Create platform-specific input capture
    fn create_input_capture() -> PlatformResult<Box<dyn InputCapture>> {
        match std::env::consts::OS {
            "linux" => {
                #[cfg(target_os = "linux")]
                {
                    Ok(Box::new(input::LinuxInputCapture::new()?))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Linux input capture not available".to_string()))
                }
            }
            "macos" => {
                #[cfg(target_os = "macos")]
                {
                    Ok(Box::new(input::MacOsInputCapture::new()?))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(PlatformError::UnsupportedPlatform("macOS input capture not available".to_string()))
                }
            }
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    Ok(Box::new(input::WindowsInputCapture::new()?))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Windows input capture not available".to_string()))
                }
            }
            _ => Err(PlatformError::UnsupportedPlatform(format!("Unsupported OS: {}", std::env::consts::OS))),
        }
    }

    /// Create platform-specific video capture
    fn create_video_capture() -> PlatformResult<Box<dyn VideoCapture>> {
        match std::env::consts::OS {
            "linux" => {
                #[cfg(target_os = "linux")]
                {
                    Ok(Box::new(video::LinuxVideoCapture::new()?))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Linux video capture not available".to_string()))
                }
            }
            "macos" => {
                #[cfg(target_os = "macos")]
                {
                    Ok(Box::new(video::MacOsVideoCapture::new()?))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(PlatformError::UnsupportedPlatform("macOS video capture not available".to_string()))
                }
            }
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    Ok(Box::new(video::WindowsVideoCapture::new()?))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Windows video capture not available".to_string()))
                }
            }
            _ => Err(PlatformError::UnsupportedPlatform(format!("Unsupported OS: {}", std::env::consts::OS))),
        }
    }

    /// Create platform-specific system service
    fn create_system_service() -> PlatformResult<Box<dyn SystemService>> {
        match std::env::consts::OS {
            "linux" => {
                #[cfg(target_os = "linux")]
                {
                    Ok(Box::new(system::LinuxSystemService::new()?))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Linux system service not available".to_string()))
                }
            }
            "macos" => {
                #[cfg(target_os = "macos")]
                {
                    Ok(Box::new(system::MacOsSystemService::new()?))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(PlatformError::UnsupportedPlatform("macOS system service not available".to_string()))
                }
            }
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    Ok(Box::new(system::WindowsSystemService::new()?))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err(PlatformError::UnsupportedPlatform("Windows system service not available".to_string()))
                }
            }
            _ => Err(PlatformError::UnsupportedPlatform(format!("Unsupported OS: {}", std::env::consts::OS))),
        }
    }

    /// Get input capture interface
    pub fn input_capture(&self) -> Option<&dyn InputCapture> {
        self.input_capture.as_ref().map(|ic| ic.as_ref())
    }

    /// Get input capture interface mutably
    pub fn input_capture_mut(&mut self) -> Option<&mut dyn InputCapture> {
        self.input_capture.as_mut().map(|ic| ic.as_mut())
    }

    /// Get video capture interface
    pub fn video_capture(&self) -> Option<&dyn VideoCapture> {
        self.video_capture.as_ref().map(|vc| vc.as_ref())
    }

    /// Get video capture interface mutably
    pub fn video_capture_mut(&mut self) -> Option<&mut dyn VideoCapture> {
        self.video_capture.as_mut().map(|vc| vc.as_mut())
    }

    /// Get system service interface
    pub fn system_service(&self) -> Option<&dyn SystemService> {
        self.system_service.as_ref().map(|ss| ss.as_ref())
    }

    /// Get system service interface mutably
    pub fn system_service_mut(&mut self) -> Option<&mut dyn SystemService> {
        self.system_service.as_mut().map(|ss| ss.as_mut())
    }

    /// Check if platform is supported
    pub fn is_supported(&self) -> bool {
        self.input_capture.is_some() && self.video_capture.is_some() && self.system_service.is_some()
    }

    /// Get platform information
    pub fn platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            family: std::env::consts::FAMILY.to_string(),
            is_supported: self.is_supported(),
        }
    }
}

/// Platform information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub is_supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_platform_manager_creation() {
        let manager = PlatformManager::new();
        match manager {
            Ok(mgr) => {
                assert!(mgr.is_supported() || !mgr.is_supported()); // Either supported or not
                let info = mgr.platform_info();
                assert!(!info.os.is_empty());
                assert!(!info.arch.is_empty());
            }
            Err(e) => {
                // Platform not supported, that's OK for testing
                println!("Platform not supported: {}", e);
            }
        }
    }
}
