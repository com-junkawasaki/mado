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
// See the License for the governing permissions and
// limitations under the License.

//! # Soft KVM Protocol
//!
//! KVM共有プロトコル実装
//!
//! ## Merkle DAG Node
//! hash: sha256:protocol_v1
//! dependencies: [core, security]

pub mod messages;
pub mod transport;
pub mod session;
pub mod video;
pub mod input;
pub mod control;

pub use messages::*;
pub use transport::*;
pub use session::*;
pub use video::*;
pub use input::*;
pub use control::*;

// Protocol constants
pub const PROTOCOL_VERSION: &str = "1.0.0";
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024; // 64MB
pub const HEARTBEAT_INTERVAL_MS: u64 = 1000;
pub const CONNECTION_TIMEOUT_MS: u64 = 5000;
