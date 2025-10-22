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

//! # Soft KVM Core
//!
//! 共通型定義とユーティリティを提供するコアライブラリ
//!
//! ## Merkle DAG Node
//! hash: sha256:core_v1
//! dependencies: []

pub mod error;
pub mod types;
pub mod utils;

pub use error::*;
pub use types::*;
pub use utils::*;
