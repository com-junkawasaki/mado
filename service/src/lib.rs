//! # Soft KVM Service
//!
//! システムサービス統合実装
//!
//! ## Merkle DAG Node
//! hash: sha256:service_v1
//! dependencies: [core, discovery, protocol, platform]

pub mod manager;
pub mod config;

#[cfg(target_os = "linux")]
pub mod systemd;
#[cfg(target_os = "macos")]
pub mod launchd;
#[cfg(target_os = "windows")]
pub mod windows_svc;

pub use manager::*;
pub use config::*;
