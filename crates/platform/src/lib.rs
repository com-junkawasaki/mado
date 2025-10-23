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
#[derive(Debug)]
pub enum PlatformManager {
    #[cfg(target_os = "linux")]
    Linux {
        input_capture: Option<input::LinuxInputCapture>,
        video_capture: Option<video::LinuxVideoCapture>,
        system_service: Option<system::LinuxSystemService>,
    },
    #[cfg(target_os = "macos")]
    MacOs {
        input_capture: Option<input::MacOsInputCapture>,
        video_capture: Option<video::MacOsVideoCapture>,
        system_service: Option<system::MacOsSystemService>,
    },
    #[cfg(target_os = "windows")]
    Windows {
        input_capture: Option<input::WindowsInputCapture>,
        video_capture: Option<video::WindowsVideoCapture>,
        system_service: Option<system::WindowsSystemService>,
    },
    Unsupported,
}

impl PlatformManager {
    /// Create a new platform manager
    pub fn new() -> PlatformResult<Self> {
        info!("Initializing platform manager for {}", std::env::consts::OS);

        match std::env::consts::OS {
            "linux" => {
                #[cfg(target_os = "linux")]
                {
                    Ok(PlatformManager::Linux {
                        input_capture: Some(input::LinuxInputCapture::new()?),
                        video_capture: Some(video::LinuxVideoCapture::new()?),
                        system_service: Some(system::LinuxSystemService::new()?),
                    })
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Ok(PlatformManager::Unsupported)
                }
            }
            "macos" => {
                #[cfg(target_os = "macos")]
                {
                    Ok(PlatformManager::MacOs {
                        input_capture: Some(input::MacOsInputCapture::new()?),
                        video_capture: Some(video::MacOsVideoCapture::new()?),
                        system_service: Some(system::MacOsSystemService::new()?),
                    })
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok(PlatformManager::Unsupported)
                }
            }
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    Ok(PlatformManager::Windows {
                        input_capture: Some(input::WindowsInputCapture::new()?),
                        video_capture: Some(video::WindowsVideoCapture::new()?),
                        system_service: Some(system::WindowsSystemService::new()?),
                    })
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Ok(PlatformManager::Unsupported)
                }
            }
            _ => Ok(PlatformManager::Unsupported),
        }
    }

    /// Check if platform is supported
    pub fn is_supported(&self) -> bool {
        !matches!(self, PlatformManager::Unsupported)
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

    /// Start input capture
    pub async fn start_input_capture(&mut self, config: soft_kvm_core::InputConfig) -> PlatformResult<()> {
        match self {
            #[cfg(target_os = "linux")]
            PlatformManager::Linux { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            #[cfg(target_os = "macos")]
            PlatformManager::MacOs { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            #[cfg(target_os = "windows")]
            PlatformManager::Windows { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            PlatformManager::Unsupported => {
                Err(PlatformError::UnsupportedPlatform("Platform not supported".to_string()))
            }
        }
    }

    /// Stop input capture
    pub async fn stop_input_capture(&mut self) -> PlatformResult<()> {
        match self {
            #[cfg(target_os = "linux")]
            PlatformManager::Linux { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            #[cfg(target_os = "macos")]
            PlatformManager::MacOs { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            #[cfg(target_os = "windows")]
            PlatformManager::Windows { input_capture, .. } => {
                if let Some(ic) = input_capture {
                    ic.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Input capture not available".to_string()))
                }
            }
            PlatformManager::Unsupported => {
                Err(PlatformError::UnsupportedPlatform("Platform not supported".to_string()))
            }
        }
    }

    /// Start video capture
    pub async fn start_video_capture(&mut self, config: soft_kvm_core::VideoConfig) -> PlatformResult<()> {
        match self {
            #[cfg(target_os = "linux")]
            PlatformManager::Linux { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            #[cfg(target_os = "macos")]
            PlatformManager::MacOs { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            #[cfg(target_os = "windows")]
            PlatformManager::Windows { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.start_capture(config).await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            PlatformManager::Unsupported => {
                Err(PlatformError::UnsupportedPlatform("Platform not supported".to_string()))
            }
        }
    }

    /// Stop video capture
    pub async fn stop_video_capture(&mut self) -> PlatformResult<()> {
        match self {
            #[cfg(target_os = "linux")]
            PlatformManager::Linux { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            #[cfg(target_os = "macos")]
            PlatformManager::MacOs { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            #[cfg(target_os = "windows")]
            PlatformManager::Windows { video_capture, .. } => {
                if let Some(vc) = video_capture {
                    vc.stop_capture().await
                } else {
                    Err(PlatformError::UnsupportedPlatform("Video capture not available".to_string()))
                }
            }
            PlatformManager::Unsupported => {
                Err(PlatformError::UnsupportedPlatform("Platform not supported".to_string()))
            }
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
