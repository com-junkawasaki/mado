//! セッション管理

use crate::{Transport, MessageSender, MessageReceiver, ProtocolMessage, MessageType, ProtocolError};
use crate::{HelloMessage, WelcomeMessage, ClientCapabilities, ServerCapabilities};
use soft_kvm_core::{ServiceId, NetworkAddress, Resolution, VideoQuality};
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// セッション状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Handshaking,
    Active,
    Error,
    Closed,
}

/// セッション情報
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: u32,
    pub peer_id: Option<ServiceId>,
    pub peer_address: NetworkAddress,
    pub protocol_version: String,
    pub capabilities: SessionCapabilities,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

impl SessionInfo {
    pub fn new(session_id: u32, peer_address: NetworkAddress) -> Self {
        let now = std::time::Instant::now();
        Self {
            session_id,
            peer_id: None,
            peer_address,
            protocol_version: "unknown".to_string(),
            capabilities: SessionCapabilities::default(),
            created_at: now,
            last_activity: now,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_activity.elapsed() > timeout
    }
}

/// セッション機能
#[derive(Debug, Clone)]
pub struct SessionCapabilities {
    pub supports_video: bool,
    pub supports_input: bool,
    pub supported_resolutions: Vec<Resolution>,
    pub supported_qualities: Vec<VideoQuality>,
    pub max_clients: usize,
}

impl Default for SessionCapabilities {
    fn default() -> Self {
        Self {
            supports_video: false,
            supports_input: false,
            supported_resolutions: vec![Resolution::fhd()],
            supported_qualities: vec![VideoQuality::balanced()],
            max_clients: 1,
        }
    }
}

/// KVMセッション
pub struct KvmSession {
    session_info: Arc<RwLock<SessionInfo>>,
    transport: Arc<RwLock<Transport>>,
    state: Arc<RwLock<SessionState>>,
    message_sender: MessageSender,
    stats: Arc<RwLock<SessionStats>>,
}

impl KvmSession {
    /// 新しいセッションを作成
    pub fn new(
        session_id: u32,
        peer_address: NetworkAddress,
        transport: Transport,
    ) -> Self {
        let session_info = SessionInfo::new(session_id, peer_address);
        let (message_sender, _) = transport.create_channel();

        Self {
            session_info: Arc::new(RwLock::new(session_info)),
            transport: Arc::new(RwLock::new(transport)),
            state: Arc::new(RwLock::new(SessionState::Connecting)),
            message_sender,
            stats: Arc::new(RwLock::new(SessionStats::new())),
        }
    }

    /// セッション状態を取得
    pub async fn get_state(&self) -> SessionState {
        *self.state.read().await
    }

    /// セッション情報を取得
    pub async fn get_info(&self) -> SessionInfo {
        self.session_info.read().await.clone()
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> SessionStats {
        futures::executor::block_on(async {
            self.stats.read().await.clone()
        })
    }

    /// セッションを開始
    pub async fn start(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.write().await;
        if *state != SessionState::Connecting {
            return Err(ProtocolError::InvalidMessage("Session already started".to_string()));
        }

        *state = SessionState::Handshaking;
        info!("Starting KVM session {}", self.session_info.read().await.session_id);

        // ハートビートを開始
        let transport = Arc::clone(&self.transport);
        let sender = self.message_sender.clone();
        tokio::spawn(async move {
            let mut transport_guard = transport.write().await;
            if let Err(e) = transport_guard.start_heartbeat(&sender).await {
                error!("Failed to start heartbeat: {}", e);
            }
        });

        // メッセージストリームを開始
        let transport = Arc::clone(&self.transport);
        let sender = self.message_sender.clone();
        let stats = Arc::clone(&self.stats);
        tokio::spawn(async move {
            let mut transport_guard = transport.write().await;
            if let Err(e) = transport_guard.start_message_stream(sender).await {
                error!("Failed to start message stream: {}", e);
            }
        });

        Ok(())
    }

    /// ハンドシェイクを実行（クライアント側）
    pub async fn perform_client_handshake(&self, client_name: &str) -> Result<(), ProtocolError> {
        let mut state = self.state.write().await;
        if *state != SessionState::Handshaking {
            return Err(ProtocolError::InvalidMessage("Not in handshaking state".to_string()));
        }

        info!("Performing client handshake as '{}'", client_name);

        // Helloメッセージを送信
        let hello = {
            let mut transport = self.transport.write().await;
            let mut builder = transport.message_builder.lock().unwrap();
            builder.build_hello(client_name)
        };

        self.message_sender.send(hello)?;

        // Welcomeメッセージを待機
        let welcome = self.wait_for_message(MessageType::Welcome, Duration::from_secs(10)).await?;

        // Welcomeメッセージをパース
        let welcome_msg: WelcomeMessage = prost::Message::decode(&welcome.payload[..])?;

        // セッション情報を更新
        {
            let mut info = self.session_info.write().await;
            info.protocol_version = super::PROTOCOL_VERSION.to_string();
            info.session_id = welcome_msg.session_id;

            if let Some(caps) = welcome_msg.server_capabilities {
                info.capabilities = SessionCapabilities {
                    supports_video: caps.supports_video,
                    supports_input: caps.supports_input,
                    supported_resolutions: caps.current_resolution
                        .map(|r| vec![r.into()])
                        .unwrap_or_default(),
                    supported_qualities: caps.current_quality
                        .map(|q| vec![q.into()])
                        .unwrap_or_default(),
                    max_clients: caps.max_clients as usize,
                };
            }

            info.update_activity();
        }

        *state = SessionState::Active;
        info!("Client handshake completed, session {} is now active", welcome_msg.session_id);

        Ok(())
    }

    /// ハンドシェイクを実行（サーバー側）
    pub async fn perform_server_handshake(&self, server_name: &str) -> Result<(), ProtocolError> {
        let mut state = self.state.write().await;
        if *state != SessionState::Handshaking {
            return Err(ProtocolError::InvalidMessage("Not in handshaking state".to_string()));
        }

        info!("Performing server handshake as '{}'", server_name);

        // Helloメッセージを待機
        let hello = self.wait_for_message(MessageType::Hello, Duration::from_secs(10)).await?;

        // Helloメッセージをパース
        let hello_msg: HelloMessage = prost::Message::decode(&hello.payload[..])?;

        // プロトコルバージョンチェック
        if hello_msg.protocol_version != super::PROTOCOL_VERSION {
            warn!("Protocol version mismatch: client={}, server={}",
                  hello_msg.protocol_version, super::PROTOCOL_VERSION);
            return Err(ProtocolError::VersionMismatch);
        }

        // セッション情報を更新
        {
            let mut info = self.session_info.write().await;
            info.protocol_version = hello_msg.protocol_version;

            if let Some(caps) = hello_msg.capabilities {
                info.capabilities = SessionCapabilities {
                    supports_video: caps.supports_video,
                    supports_input: caps.supports_input,
                    supported_resolutions: caps.supported_resolutions
                        .into_iter()
                        .map(|r| r.into())
                        .collect(),
                    supported_qualities: caps.supported_qualities
                        .into_iter()
                        .map(|q| q.into())
                        .collect(),
                    max_clients: 1, // サーバー側は固定
                };
            }

            info.update_activity();
        }

        // Welcomeメッセージを送信
        let welcome = {
            let mut transport = self.transport.write().await;
            let session_id = transport.message_builder.lock().unwrap().next_sequence();
            let mut builder = transport.message_builder.lock().unwrap();
            builder.build_welcome(server_name, session_id)
        };

        self.message_sender.send(welcome)?;

        *state = SessionState::Active;
        info!("Server handshake completed, session is now active");

        Ok(())
    }

    /// 指定タイプのメッセージを待機
    async fn wait_for_message(&self, message_type: MessageType, timeout_duration: Duration) -> Result<ProtocolMessage, ProtocolError> {
        let start_time = Instant::now();

        loop {
            // タイムアウトチェック
            if start_time.elapsed() > timeout_duration {
                return Err(ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Timeout waiting for {:?} message", message_type)
                )));
            }

            // メッセージを受信を試行
            let mut transport = self.transport.write().await;
            match timeout(Duration::from_millis(100), transport.receive_message()).await {
                Ok(Ok(message)) => {
                    if message.message_type == message_type {
                        return Ok(message);
                    } else {
                        debug!("Received unexpected message type: {:?}", message.message_type);
                        // 他のメッセージは無視して継続
                    }
                }
                Ok(Err(e)) => {
                    debug!("Error receiving message: {}", e);
                }
                Err(_) => {
                    // タイムアウトは継続
                }
            }
        }
    }

    /// メッセージを送信
    pub async fn send_message(&self, message: ProtocolMessage) -> Result<(), ProtocolError> {
        let mut transport = self.transport.write().await;
        transport.send_message(message).await?;

        // 統計更新
        {
            let mut stats = self.stats.write().await;
            stats.record_sent_message();
        }

        Ok(())
    }

    /// セッションを閉じる
    pub async fn close(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.write().await;
        if *state == SessionState::Closed {
            return Ok(());
        }

        info!("Closing KVM session {}", self.session_info.read().await.session_id);

        // Goodbyeメッセージを送信
        let goodbye = {
            let mut transport = self.transport.write().await;
            let mut builder = transport.message_builder.lock().unwrap();
            builder.build_heartbeat() // 簡易的にheartbeatを使う（実際はgoodbyeを実装）
        };

        let _ = self.message_sender.send(goodbye); // エラーは無視

        // トランスポートを閉じる
        let mut transport = self.transport.write().await;
        let _ = transport.close().await; // エラーは無視

        *state = SessionState::Closed;
        info!("KVM session closed");

        Ok(())
    }

    /// セッションがアクティブかチェック
    pub async fn is_active(&self) -> bool {
        *self.state.read().await == SessionState::Active
    }

    /// セッションが期限切れかチェック
    pub async fn is_expired(&self, timeout: Duration) -> bool {
        let info = self.session_info.read().await;
        info.is_expired(timeout)
    }
}

/// セッション統計
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub created_at: std::time::Instant,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            errors: 0,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn record_sent_message(&mut self) {
        self.messages_sent += 1;
    }

    pub fn record_received_message(&mut self) {
        self.messages_received += 1;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn duration(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

/// セッションマネージャー
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<u32, Arc<KvmSession>>>>,
    next_session_id: Arc<RwLock<u32>>,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new(session_timeout: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(RwLock::new(1)),
            session_timeout,
        }
    }

    /// 新しいセッションを作成
    pub async fn create_session(
        &self,
        peer_address: NetworkAddress,
        transport: Transport,
    ) -> Result<Arc<KvmSession>, ProtocolError> {
        let session_id = {
            let mut next_id = self.next_session_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let session = Arc::new(KvmSession::new(session_id, peer_address, transport));

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, Arc::clone(&session));
        }

        info!("Created new session {}", session_id);
        Ok(session)
    }

    /// セッションを取得
    pub async fn get_session(&self, session_id: u32) -> Option<Arc<KvmSession>> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).cloned()
    }

    /// セッションを削除
    pub async fn remove_session(&self, session_id: u32) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id).is_some()
    }

    /// 期限切れセッションをクリーンアップ
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|_, session| {
            !futures::executor::block_on(session.is_expired(self.session_timeout))
        });

        let removed_count = initial_count - sessions.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired sessions", removed_count);
        }

        removed_count
    }

    /// 全セッションを取得
    pub async fn get_all_sessions(&self) -> Vec<Arc<KvmSession>> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// アクティブセッション数を取得
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| futures::executor::block_on(s.is_active()))
            .count()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 5分タイムアウト
    }
}
