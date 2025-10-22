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

//! # Soft KVM Security Plugin
//!
//! Tauri plugin for TLS and certificate management

use tauri::{plugin::Builder, plugin::TauriPlugin, Runtime, Manager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use rcgen::{Certificate, CertificateParams, KeyPair, DistinguishedName, DnType};
use chrono::{Utc, Duration};
use ring::digest;
use hex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_cert_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CertificateInfo {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
    pub fingerprint: String,
}

/// Generate a self-signed certificate (mock implementation)
fn generate_self_signed_certificate(
    common_name: &str,
    _organization: Option<&str>,
    _validity_days: u32,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    // Mock certificate generation - replace with actual rcgen implementation later
    let mock_cert = format!("-----BEGIN CERTIFICATE-----\n\
MOCK_CERTIFICATE_FOR_{}\n\
-----END CERTIFICATE-----\n", common_name);

    let mock_key = "-----BEGIN PRIVATE KEY-----\n\
MOCK_PRIVATE_KEY\n\
-----END PRIVATE KEY-----\n".to_string();

    Ok((mock_cert.as_bytes().to_vec(), mock_key.as_bytes().to_vec()))
}

/// Calculate certificate fingerprint
fn calculate_fingerprint(cert_pem: &[u8]) -> Result<String, String> {
    // Simple SHA256 fingerprint calculation
    let digest = digest::digest(&digest::SHA256, cert_pem);
    let fingerprint = hex::encode(digest.as_ref());

    // Format as colon-separated hex
    Ok(fingerprint.chars()
        .collect::<Vec<char>>()
        .chunks(2)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join(":"))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SecurityStatus {
    pub tls_initialized: bool,
    pub certificates_loaded: bool,
    pub total_handshakes: u64,
    pub successful_handshakes: u64,
    pub failed_handshakes: u64,
}

#[derive(Debug)]
struct SecurityState {
    tls_config: Option<TlsConfig>,
    certificates: Vec<(Vec<u8>, Vec<u8>)>, // (cert_pem, key_pem)
}

impl Default for SecurityState {
    fn default() -> Self {
        SecurityState {
            tls_config: None,
            certificates: Vec::new(),
        }
    }
}

/// Initialize TLS configuration
#[tauri::command]
async fn init_tls(
    config: TlsConfig,
    state: tauri::State<'_, Arc<RwLock<SecurityState>>>,
) -> Result<String, String> {
    let mut security_state = state.write().await;

    println!("Initializing TLS with config: {:?}", config);

    security_state.tls_config = Some(config);

    Ok("TLS initialized successfully".to_string())
}

/// Generate self-signed certificate
#[tauri::command]
async fn generate_certificate(
    common_name: String,
    state: tauri::State<'_, Arc<RwLock<SecurityState>>>,
) -> Result<CertificateInfo, String> {
    println!("Generating certificate for: {}", common_name);

    // Generate self-signed certificate
    let (cert_pem, key_pem) = generate_self_signed_certificate(
        &common_name,
        Some("Soft KVM"),
        365, // 1 year validity
    )?;

    // Calculate fingerprint
    let fingerprint = calculate_fingerprint(&cert_pem)?;

    // Store certificate in state
    let mut security_state = state.write().await;
    security_state.certificates.push((cert_pem.clone(), key_pem));

    Ok(CertificateInfo {
        subject: common_name.clone(),
        issuer: common_name.clone(),
        valid_from: Utc::now().format("%Y-%m-%d").to_string(),
        valid_until: (Utc::now() + Duration::days(365)).format("%Y-%m-%d").to_string(),
        fingerprint: fingerprint.chars()
            .collect::<Vec<char>>()
            .chunks(2)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join(":"),
    })
}

/// Validate certificate
#[tauri::command]
async fn validate_certificate(
    cert_data: String,
    _state: tauri::State<'_, Arc<RwLock<SecurityState>>>,
) -> Result<bool, String> {
    println!("Validating certificate");

    // Simple validation - just check if it's not empty and contains valid PEM structure
    let cert_data = cert_data.trim();
    if cert_data.is_empty() {
        return Ok(false);
    }

    // Check for basic PEM structure
    let has_begin = cert_data.contains("-----BEGIN");
    let has_end = cert_data.contains("-----END");

    Ok(has_begin && has_end)
}

/// Get security status
#[tauri::command]
async fn get_security_status(
    state: tauri::State<'_, Arc<RwLock<SecurityState>>>,
) -> Result<SecurityStatus, String> {
    let security_state = state.read().await;

    let tls_initialized = security_state.tls_config.is_some();
    let certificates_loaded = !security_state.certificates.is_empty();

    Ok(SecurityStatus {
        tls_initialized,
        certificates_loaded,
        total_handshakes: 0,
        successful_handshakes: 0,
        failed_handshakes: 0,
    })
}

/// List stored certificates
#[tauri::command]
async fn list_certificates(
    state: tauri::State<'_, Arc<RwLock<SecurityState>>>,
) -> Result<Vec<CertificateInfo>, String> {
    let security_state = state.read().await;

    let mut certificates = Vec::new();

    // Create certificate info from stored certificates
    for (i, (cert_pem, _)) in security_state.certificates.iter().enumerate() {
        let fingerprint = calculate_fingerprint(cert_pem)
            .unwrap_or_else(|_| "00:11:22:33:44:55".to_string());

        certificates.push(CertificateInfo {
            subject: format!("Certificate {}", i + 1),
            issuer: "Soft KVM".to_string(),
            valid_from: Utc::now().format("%Y-%m-%d").to_string(),
            valid_until: (Utc::now() + Duration::days(365)).format("%Y-%m-%d").to_string(),
            fingerprint: fingerprint.chars()
                .collect::<Vec<char>>()
                .chunks(2)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<String>>()
                .join(":"),
        });
    }

    Ok(certificates)
}

/// Initialize the security plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("soft-kvm-security")
        .invoke_handler(tauri::generate_handler![
            init_tls,
            generate_certificate,
            validate_certificate,
            get_security_status,
            list_certificates,
        ])
        .setup(|_app| {
            // Initialize security state
            let state = Arc::new(RwLock::new(SecurityState::default()));
            _app.manage(state);
            Ok(())
        })
        .build()
}
