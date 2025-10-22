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

//! # Soft KVM Server
//!
//! LAN専用・低遅延KVM共有サーバー実装

use clap::{Arg, Command};
use soft_kvm_core::*;
use soft_kvm_discovery::*;
use soft_kvm_security::*;
use soft_kvm_platform::*;
use soft_kvm_service::*;
use soft_kvm_monitoring::*;
use soft_kvm_protocol::*;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use std::sync::Arc;

/// サーバー設定
#[derive(Debug, Clone)]
struct ServerConfig {
    listen_address: NetworkAddress,
    discovery_enabled: bool,
    service_name: String,
    max_clients: usize,
    video_enabled: bool,
    input_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: NetworkAddress::localhost(8080),
            discovery_enabled: true,
            service_name: "Soft KVM Server".to_string(),
            max_clients: 1,
            video_enabled: true,
            input_enabled: true,
        }
    }
}

/// KVMサーバー
struct KvmServer {
    config: ServerConfig,
    session_manager: Arc<SessionManager>,
    discovery_resolver: Option<ServiceResolver>,
    permission_manager: PermissionManager,
    metrics_collector: MetricsCollector,
}

impl KvmServer {
    async fn new(config: ServerConfig) -> KvmResult<Self> {
        // 権限チェック
        let mut permission_manager = PermissionManager::new();
        permission_manager.request_all_permissions().await?;

        // セッションマネージャー初期化
        let session_manager = Arc::new(SessionManager::default());

        // mDNSディスカバリ初期化
        let discovery_resolver = if config.discovery_enabled {
            let resolver = ServiceResolver::new(ServiceType::Server);
            resolver.start_discovery().await?;
            Some(resolver)
        } else {
            None
        };

        // メトリクス収集初期化
        let metrics_collector = MetricsCollector::new();

        Ok(Self {
            config,
            session_manager,
            discovery_resolver,
            permission_manager,
            metrics_collector,
        })
    }

    /// サーバーを起動
    async fn start(&self) -> KvmResult<()> {
        info!("Starting Soft KVM Server on {}:{}", self.config.listen_address.ip, self.config.listen_address.port);

        // TCPリスナーを開始
        let listener = TcpListener::bind(format!("{}:{}", self.config.listen_address.ip, self.config.listen_address.port)).await?;

        info!("Server listening on {}:{}", self.config.listen_address.ip, self.config.listen_address.port);

        // サービスを登録
        if let Some(resolver) = &self.discovery_resolver {
            let capabilities = ServiceCapabilities::default();
            let service_info = ServiceInfo::new(ServiceType::Server, &self.config.service_name, self.config.listen_address.clone(), capabilities);
            resolver.register_service(service_info).await?;
        }

        // メインの接続受け入れループ
        let session_manager = Arc::clone(&self.session_manager);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        info!("New connection from {}", addr);

                        let session_manager = Arc::clone(&session_manager);
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_connection(socket, addr, session_manager).await {
                                error!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        // ビデオキャプチャを開始（有効な場合）
        if self.config.video_enabled {
            self.start_video_capture().await?;
        }

        // 入力処理を開始（有効な場合）
        if self.config.input_enabled {
            self.start_input_processing().await?;
        }

        // 定期的なクリーンアップを開始
        self.start_cleanup_task().await;

        info!("Soft KVM Server started successfully");
        Ok(())
    }

    /// 接続を処理
    async fn handle_connection(
        socket: tokio::net::TcpStream,
        addr: std::net::SocketAddr,
        session_manager: Arc<SessionManager>,
    ) -> KvmResult<()> {
        // クライアント数をチェック
        if session_manager.active_session_count().await >= 1 { // 現在は1クライアントのみ
            warn!("Maximum clients reached, rejecting connection from {}", addr);
            return Ok(());
        }

        // TLSハンドシェイクを実行
        let mut handshake_manager = SecureConnectionBuilder::new()
            .with_timeout(30)
            .build();

        let peer_address = NetworkAddress::new(addr.ip(), addr.port() as u16);
        let handshake_result = handshake_manager.perform_server_handshake(socket).await?;

        info!("TLS handshake completed with {}", addr);

        // トランスポート層を初期化
        let transport = TransportBuilder::new()
            .build(handshake_result.stream);

        // セッションを作成
        let session = session_manager.create_session(peer_address, transport).await?;
        session.start().await?;

        // サーバーハンドシェイクを実行
        session.perform_server_handshake("Soft KVM Server").await?;

        info!("Session {} established with {}", session.get_info().await.session_id, addr);

        // セッション処理を開始
        Self::handle_session(session).await;

        Ok(())
    }

    /// セッションを処理
    async fn handle_session(session: Arc<KvmSession>) {
        // セッションのメッセージ処理ループ
        tokio::spawn(async move {
            loop {
                if !session.is_active().await {
                    break;
                }

                // メッセージ受信と処理（簡易実装）
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // セッションクリーンアップ
            if let Err(e) = session.close().await {
                error!("Error closing session: {}", e);
            }
        });
    }

    /// ビデオキャプチャを開始
    async fn start_video_capture(&self) -> KvmResult<()> {
        info!("Starting video capture...");

        // プラットフォーム固有のビデオキャプチャを開始
        #[cfg(target_os = "linux")]
        {
            crate::platform::linux::video::start_capture().await?;
        }

        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::video::start_capture().await?;
        }

        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::video::start_capture().await?;
        }

        Ok(())
    }

    /// 入力処理を開始
    async fn start_input_processing(&self) -> KvmResult<()> {
        info!("Starting input processing...");

        // プラットフォーム固有の入力処理を開始
        #[cfg(target_os = "linux")]
        {
            crate::platform::linux::input::start_processing().await?;
        }

        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::input::start_processing().await?;
        }

        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::input::start_processing().await?;
        }

        Ok(())
    }

    /// クリーンアップタスクを開始
    async fn start_cleanup_task(&self) {
        let session_manager = Arc::clone(&self.session_manager);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;
                let cleaned = session_manager.cleanup_expired_sessions().await;
                if cleaned > 0 {
                    info!("Cleaned up {} expired sessions", cleaned);
                }
            }
        });
    }

    /// 統計情報を表示
    async fn print_stats(&self) {
        let active_sessions = self.session_manager.active_session_count().await;
        let total_sessions = self.session_manager.get_all_sessions().await.len();

        info!("Server Stats: Active={}, Total={}", active_sessions, total_sessions);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // コマンドライン引数解析
    let matches = Command::new("soft-kvm-server")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Soft KVM Team")
        .about("LAN専用・低遅延KVM共有サーバー")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Listen port")
                .default_value("8080")
        )
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .value_name("ADDRESS")
                .help("Listen address")
                .default_value("0.0.0.0")
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Server name")
                .default_value("Soft KVM Server")
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
                .help("Disable video capture")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("no-input")
                .long("no-input")
                .help("Disable input processing")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // 設定を構築
    let port: u16 = matches.get_one::<String>("port").unwrap().parse()?;
    let address: std::net::IpAddr = matches.get_one::<String>("address").unwrap().parse()?;
    let service_name = matches.get_one::<String>("name").unwrap().clone();

    let config = ServerConfig {
        listen_address: NetworkAddress::new(address, port),
        discovery_enabled: !matches.get_flag("no-discovery"),
        service_name,
        max_clients: 1,
        video_enabled: !matches.get_flag("no-video"),
        input_enabled: !matches.get_flag("no-input"),
    };

    info!("Starting Soft KVM Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration: {:?}", config);

    // サーバーを作成して起動
    let server = KvmServer::new(config).await?;
    server.start().await?;

    // 統計表示タスク
    let server_stats = Arc::new(server);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            server_stats.print_stats().await;
        }
    });

    // シグナルハンドリング
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, stopping server...");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    info!("Soft KVM Server stopped");
    Ok(())
}
