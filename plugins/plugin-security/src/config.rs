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

//! 設定管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// セキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls: TlsSecurityConfig,
    pub certificates: CertificateSecurityConfig,
    pub handshake: HandshakeSecurityConfig,
}

/// TLSセキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsSecurityConfig {
    pub enabled: bool,
    pub require_client_cert: bool,
    pub cipher_suites: Vec<String>,
    pub protocol_versions: Vec<String>,
}

/// 証明書セキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSecurityConfig {
    pub auto_generate: bool,
    pub validity_days: i64,
    pub allowed_domains: Vec<String>,
    pub require_revocation_check: bool,
}

/// ハンドシェイクセキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeSecurityConfig {
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub allow_self_signed: bool,
    pub verify_fingerprints: HashMap<String, String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            tls: TlsSecurityConfig::default(),
            certificates: CertificateSecurityConfig::default(),
            handshake: HandshakeSecurityConfig::default(),
        }
    }
}

impl Default for TlsSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_client_cert: false,
            cipher_suites: vec!["TLS13_AES_256_GCM_SHA384".to_string()],
            protocol_versions: vec!["TLSv1.3".to_string()],
        }
    }
}

impl Default for CertificateSecurityConfig {
    fn default() -> Self {
        Self {
            auto_generate: true,
            validity_days: 365,
            allowed_domains: vec!["local".to_string(), "localhost".to_string()],
            require_revocation_check: false,
        }
    }
}

impl Default for HandshakeSecurityConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            allow_self_signed: true,
            verify_fingerprints: HashMap::new(),
        }
    }
}

/// 設定ローダー
pub struct SecurityConfigLoader;

impl SecurityConfigLoader {
    /// 設定ファイルを読み込み
    pub fn load_from_file(path: &std::path::PathBuf) -> Result<SecurityConfig, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: SecurityConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// デフォルト設定を作成
    pub fn create_default() -> SecurityConfig {
        SecurityConfig::default()
    }

    /// 設定をファイルに保存
    pub fn save_to_file(config: &SecurityConfig, path: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 環境変数から設定を上書き
    pub fn override_from_env(mut config: SecurityConfig) -> SecurityConfig {
        if let Ok(enabled) = std::env::var("SOFT_KVM_TLS_ENABLED") {
            config.tls.enabled = enabled.parse().unwrap_or(true);
        }

        if let Ok(require_client) = std::env::var("SOFT_KVM_REQUIRE_CLIENT_CERT") {
            config.tls.require_client_cert = require_client.parse().unwrap_or(false);
        }

        if let Ok(timeout) = std::env::var("SOFT_KVM_HANDSHAKE_TIMEOUT") {
            config.handshake.timeout_seconds = timeout.parse().unwrap_or(30);
        }

        config
    }
}
