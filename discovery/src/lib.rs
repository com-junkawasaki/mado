//! # Soft KVM Discovery
//!
//! mDNSベースのサービスディスカバリ実装
//!
//! ## Merkle DAG Node
//! hash: sha256:discovery_v1
//! dependencies: [core]

pub mod service;
pub mod resolver;

pub use service::*;
pub use resolver::*;
