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

//! Error types for Soft KVM

use std::fmt;

/// Result type alias for Soft KVM operations
pub type KvmResult<T> = Result<T, KvmError>;

/// Main error type for Soft KVM
#[derive(Debug, thiserror::Error)]
pub enum KvmError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Video error: {0}")]
    Video(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Generic error: {0}")]
    GenericError(String),
}
