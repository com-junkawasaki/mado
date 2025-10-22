//! # Soft KVM Platform
//!
//! OS別バックエンド統合実装
//!
//! ## Merkle DAG Node
//! hash: sha256:platform_v1
//! dependencies: [core]

pub mod capture;
pub mod input;
pub mod permissions;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub use capture::*;
pub use input::*;
pub use permissions::*;
