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

//! # Soft KVM Client
//!
//! LAN専用・低遅延KVM共有クライアント実装

use clap::{Arg, Command};
use soft_kvm_core::*;
use soft_kvm_discovery::*;
use soft_kvm_security::*;
use soft_kvm_platform::*;
use soft_kvm_monitoring::*;
use soft_kvm_protocol::*;
use tokio::net::TcpStream;
use tokio::signal;
use tracing::{info, error, warn};
use std::sync::Arc;

/// クライアント設定
#[derive(Debug, Clone)]
struct ClientConfig {
    server_address: Option<NetworkAddress>,
    server_name: Option<String>,
    client_name: String,
    auto_discovery: bool,
    video_enabled: bool,
    input_enabled: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: None,
            server_name: None,
            client_name: format!("Soft KVM Client {}", std::process::id()),
            auto_discovery: true,
            video_enabled: true,
            input_enabled: true,
        }
    }
}

/// KVMクライアント
struct KvmClient {
    config: ClientConfig,
    discovery_resolver: Option<ServiceResolver>,
    metrics_collector: MetricsCollector,
}

impl KvmClient {
    async fn new(config: ClientConfig) -> KvmResult<Self> {
        // mDNSディスカバリ初期化
        let discovery_resolver = if config.auto_discovery {
            let resolver = ServiceResolver::new(ServiceType::Client);
            resolver.start_discovery().await?;
            Some(resolver)
        } else {
            None
        };

        // メトリクス収集初期化
        let metrics_collector = MetricsCollector::new();

        Ok(Self {
            config,
            discovery_resolver,
            metrics_collector,
        })
    }

    /// サーバーに接続
    async fn connect(&self) -> KvmResult<()> {
        let server_address = self.resolve_server_address().await?;

        info!("Connecting to server at {}:{}", server_address.ip, server_address.port);

        // TCP接続を確立
        let socket = TcpStream::connect(format!("{}:{}", server_address.ip, server_address.port)).await?;
        info!("TCP connection established");

        // TLSハンドシェイクを実行
        let mut handshake_manager = SecureConnectionBuilder::new()
            .with_timeout(30)
            .build();

        let handshake_result = handshake_manager.perform_client_handshake(socket, &server_address, None).await?;
        info!("TLS handshake completed");

        // トランスポート層を初期化
        let transport = TransportBuilder::new()
            .build(handshake_result.stream);

        // セッションマネージャーを作成
        let session_manager = Arc::new(SessionManager::default());

        // セッションを作成
        let session = session_manager.create_session(server_address, transport).await?;
        session.start().await?;

        // クライアントハンドシェイクを実行
        session.perform_client_handshake(&self.config.client_name).await?;
        info!("Protocol handshake completed");

        // セッション処理を開始
        self.handle_session(session).await;

        Ok(())
    }

    /// サーバーアドレスを解決
    async fn resolve_server_address(&self) -> KvmResult<NetworkAddress> {
        // 直接指定されたアドレスを使用
        if let Some(address) = &self.config.server_address {
            return Ok(address.clone());
        }

        // サービス名からのディスカバリ
        if let Some(resolver) = &self.discovery_resolver {
            if let Some(server_name) = &self.config.server_name {
                let services = resolver.get_available_services().await;
                for service in services {
                    if service.name == *server_name && service.service_type == ServiceType::Server {
                        info!("Found server '{}' via mDNS at {}", server_name, service.address.ip);
                        return Ok(service.address);
                    }
                }
                warn!("Server '{}' not found via mDNS", server_name);
            }
        }

        // デフォルトアドレスを使用
        warn!("Using default server address localhost:8080");
        Ok(NetworkAddress::localhost(8080))
    }

    /// セッションを処理
    async fn handle_session(&self, session: Arc<KvmSession>) {
        let session_info = session.get_info().await;
        info!("Session {} started", session_info.session_id);

        // ビデオ表示を開始（有効な場合）
        if self.config.video_enabled {
            self.start_video_display(&session).await;
        }

        // 入力送信を開始（有効な場合）
        if self.config.input_enabled {
            self.start_input_sending(&session).await;
        }

        // セッション監視ループ
        tokio::spawn(async move {
            loop {
                if !session.is_active().await {
                    info!("Session ended");
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // 統計表示
                let stats = session.get_stats();
                debug!("Session stats: sent={}, received={}",
                      stats.messages_sent, stats.messages_received);
            }
        });
    }

    /// ビデオ表示を開始
    async fn start_video_display(&self, _session: &Arc<KvmSession>) {
        info!("Starting video display...");

        tokio::spawn(async move {
            // プラットフォーム固有のビデオ表示を開始
            #[cfg(target_os = "linux")]
            {
                if let Err(e) = crate::platform::linux::video::start_display().await {
                    error!("Failed to start video display: {}", e);
                }
            }

            #[cfg(target_os = "macos")]
            {
                if let Err(e) = crate::platform::macos::video::start_display().await {
                    error!("Failed to start video display: {}", e);
                }
            }

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = crate::platform::windows::video::start_display().await {
                    error!("Failed to start video display: {}", e);
                }
            }
        });
    }

    /// 入力送信を開始
    async fn start_input_sending(&self, _session: &Arc<KvmSession>) {
        info!("Starting input sending...");

        tokio::spawn(async move {
            // プラットフォーム固有の入力キャプチャを開始
            #[cfg(target_os = "linux")]
            {
                if let Err(e) = crate::platform::linux::input::start_capture().await {
                    error!("Failed to start input capture: {}", e);
                }
            }

            #[cfg(target_os = "macos")]
            {
                if let Err(e) = crate::platform::macos::input::start_capture().await {
                    error!("Failed to start input capture: {}", e);
                }
            }

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = crate::platform::windows::input::start_capture().await {
                    error!("Failed to start input capture: {}", e);
                }
            }
        });
    }

    /// 利用可能なサーバーをリスト
    async fn list_servers(&self) -> KvmResult<()> {
        if let Some(resolver) = &self.discovery_resolver {
            let services = resolver.get_available_services().await;
            println!("Available servers:");

            for service in services {
                if service.service_type == ServiceType::Server {
                    println!("  - {} at {}:{} ({})",
                            service.name,
                            service.address.ip,
                            service.address.port,
                            if service.is_expired() { "expired" } else { "active" });
                }
            }

            if services.is_empty() {
                println!("  No servers found");
            }
        } else {
            println!("mDNS discovery not enabled");
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // コマンドライン引数解析
    let matches = Command::new("soft-kvm-client")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Soft KVM Team")
        .about("LAN専用・低遅延KVM共有クライアント")
        .subcommand_required(false)
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("ADDRESS:PORT")
                .help("Server address and port")
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Server name (for mDNS discovery)")
        )
        .arg(
            Arg::new("client-name")
                .long("client-name")
                .value_name("NAME")
                .help("Client name")
                .default_value("Soft KVM Client")
        )
        .arg(
            Arg::new("no-discovery")
                .long("no-discovery")
                .help("Disable mDNS discovery")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("no-video")
                .long("no-video")
                .help("Disable video display")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("no-input")
                .long("no-input")
                .help("Disable input sending")
                .action(clap::ArgAction::SetTrue)
        )
        .subcommand(
            Command::new("list")
                .about("List available servers")
        )
        .get_matches();

    // 設定を構築
    let server_address = matches.get_one::<String>("server")
        .map(|addr| {
            let parts: Vec<&str> = addr.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(ip), Ok(port)) = (parts[0].parse(), parts[1].parse()) {
                    Some(NetworkAddress::new(ip, port))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .flatten();

    let server_name = matches.get_one::<String>("name").cloned();
    let client_name = matches.get_one::<String>("client-name").unwrap().clone();

    let config = ClientConfig {
        server_address,
        server_name,
        client_name,
        auto_discovery: !matches.get_flag("no-discovery"),
        video_enabled: !matches.get_flag("no-video"),
        input_enabled: !matches.get_flag("no-input"),
    };

    info!("Starting Soft KVM Client v{}", env!("CARGO_PKG_VERSION"));

    // サブコマンド処理
    if let Some(_) = matches.subcommand_matches("list") {
        let client = KvmClient::new(config).await?;
        return client.list_servers().await.map_err(Into::into);
    }

    // 通常の接続処理
    info!("Configuration: {:?}", config);

    let client = KvmClient::new(config).await?;
    client.connect().await?;

    // シグナルハンドリング
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, stopping client...");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    info!("Soft KVM Client stopped");
    Ok(())
}
