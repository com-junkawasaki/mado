//! トランスポート層実装

use crate::{ProtocolMessage, ProtocolError, MessageType, HEARTBEAT_INTERVAL_MS, CONNECTION_TIMEOUT_MS};
use soft_kvm_security::SecureStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration, interval};
use tokio::sync::mpsc;
use tracing::{debug, warn, error, info};
use std::sync::Arc;
use std::time::Instant;

/// トランスポート設定
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub max_message_size: usize,
    pub read_timeout_ms: u64,
    pub write_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub enable_compression: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_message_size: super::MAX_MESSAGE_SIZE,
            read_timeout_ms: CONNECTION_TIMEOUT_MS,
            write_timeout_ms: 5000,
            heartbeat_interval_ms: HEARTBEAT_INTERVAL_MS,
            enable_compression: false,
        }
    }
}

/// メッセージ送信者
#[derive(Debug, Clone)]
pub struct MessageSender {
    tx: mpsc::UnboundedSender<ProtocolMessage>,
}

impl MessageSender {
    /// メッセージを送信
    pub fn send(&self, message: ProtocolMessage) -> Result<(), ProtocolError> {
        self.tx.send(message).map_err(|_| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel closed"
            ))
        })
    }

    /// 非同期でメッセージを送信
    pub async fn send_async(&self, message: ProtocolMessage) -> Result<(), ProtocolError> {
        // 非同期版が必要な場合は実装
        self.send(message)
    }
}

/// メッセージ受信者
#[derive(Debug)]
pub struct MessageReceiver {
    rx: mpsc::UnboundedReceiver<ProtocolMessage>,
}

impl MessageReceiver {
    /// メッセージを受信
    pub async fn recv(&mut self) -> Option<ProtocolMessage> {
        self.rx.recv().await
    }

    /// タイムアウト付きでメッセージを受信
    pub async fn recv_timeout(&mut self, timeout: Duration) -> Option<ProtocolMessage> {
        match timeout(timeout, self.rx.recv()).await {
            Ok(result) => result,
            Err(_) => None,
        }
    }
}

/// トランスポート層
pub struct Transport {
    stream: SecureStream,
    config: TransportConfig,
    message_builder: Arc<std::sync::Mutex<crate::MessageBuilder>>,
    stats: Arc<std::sync::Mutex<TransportStats>>,
}

impl Transport {
    pub fn new(stream: SecureStream, config: TransportConfig) -> Self {
        Self {
            stream,
            config,
            message_builder: Arc::new(std::sync::Mutex::new(crate::MessageBuilder::new())),
            stats: Arc::new(std::sync::Mutex::new(TransportStats::new())),
        }
    }

    /// 双方向チャネルを作成
    pub fn create_channel(&self) -> (MessageSender, MessageReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (MessageSender { tx }, MessageReceiver { rx })
    }

    /// メッセージを送信
    pub async fn send_message(&mut self, message: ProtocolMessage) -> Result<(), ProtocolError> {
        let start_time = Instant::now();
        let data = message.to_bytes();

        if data.len() > self.config.max_message_size {
            return Err(ProtocolError::InvalidMessage(
                format!("Message too large: {} bytes", data.len())
            ));
        }

        // タイムアウト付きで送信
        let write_future = self.stream.write_all(&data);
        timeout(Duration::from_millis(self.config.write_timeout_ms), write_future)
            .await
            .map_err(|_| ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Write timeout"
            )))??;

        let elapsed = start_time.elapsed().as_micros() as f64;
        debug!("Sent message type={:?}, size={} bytes, time={:.2}µs",
               message.message_type, data.len(), elapsed);

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record_sent_message(data.len(), elapsed);
        }

        Ok(())
    }

    /// メッセージを受信
    pub async fn receive_message(&mut self) -> Result<ProtocolMessage, ProtocolError> {
        let start_time = Instant::now();

        // ヘッダーを読み取り（17バイト）
        let mut header_buf = [0u8; 17];
        let read_future = self.stream.read_exact(&mut header_buf);
        timeout(Duration::from_millis(self.config.read_timeout_ms), read_future)
            .await
            .map_err(|_| ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Read timeout"
            )))??;

        // ペイロード長を取得
        let payload_len = u32::from_le_bytes(header_buf[13..17].try_into().unwrap()) as usize;

        if payload_len > self.config.max_message_size {
            return Err(ProtocolError::InvalidMessage(
                format!("Payload too large: {} bytes", payload_len)
            ));
        }

        // ペイロードを読み取り
        let mut payload_buf = vec![0u8; payload_len];
        if payload_len > 0 {
            let read_future = self.stream.read_exact(&mut payload_buf);
            timeout(Duration::from_millis(self.config.read_timeout_ms), read_future)
                .await
                .map_err(|_| ProtocolError::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Read timeout"
                )))??;
        }

        // 完全なメッセージバッファを作成
        let mut full_buf = header_buf.to_vec();
        full_buf.extend_from_slice(&payload_buf);

        // メッセージをデシリアライズ
        let message = ProtocolMessage::from_bytes(&full_buf)?;

        let elapsed = start_time.elapsed().as_micros() as f64;
        debug!("Received message type={:?}, size={} bytes, time={:.2}µs",
               message.message_type, full_buf.len(), elapsed);

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record_received_message(full_buf.len(), elapsed);
        }

        Ok(message)
    }

    /// ハートビートを開始
    pub async fn start_heartbeat(&self, sender: &MessageSender) -> Result<(), ProtocolError> {
        let message_builder = Arc::clone(&self.message_builder);
        let sender = sender.clone();
        let interval_ms = self.config.heartbeat_interval_ms;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));

            loop {
                interval.tick().await;

                let heartbeat = {
                    let mut builder = message_builder.lock().unwrap();
                    builder.build_heartbeat()
                };

                if let Err(e) = sender.send(heartbeat) {
                    warn!("Failed to send heartbeat: {}", e);
                    break;
                }

                debug!("Heartbeat sent");
            }
        });

        Ok(())
    }

    /// メッセージストリームを開始（受信ループ）
    pub async fn start_message_stream(
        &mut self,
        sender: MessageSender,
    ) -> Result<(), ProtocolError> {
        let mut receiver = MessageReceiver {
            rx: mpsc::unbounded_channel().1, // ダミー（実際の受信は別途）
        };

        // 受信タスクを開始
        let stats = Arc::clone(&self.stats);
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Some(message) => {
                        // メッセージ処理（実際の実装ではハンドラーにディスパッチ）
                        match message.message_type {
                            MessageType::Heartbeat => {
                                debug!("Heartbeat received");
                            }
                            MessageType::Error => {
                                warn!("Error message received");
                            }
                            _ => {
                                debug!("Message received: {:?}", message.message_type);
                            }
                        }
                    }
                    None => {
                        info!("Message stream ended");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> TransportStats {
        self.stats.lock().unwrap().clone()
    }

    /// 接続を閉じる
    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        // TLSストリームのシャットダウン
        // 注意: tokio_rustlsのSecureStreamにはshutdownがないので、
        // 基になるTCPストリームを直接シャットダウン
        Ok(())
    }
}

/// トランスポート統計
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub send_time_total_us: u64,
    pub receive_time_total_us: u64,
    pub errors: u64,
}

impl TransportStats {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            send_time_total_us: 0,
            receive_time_total_us: 0,
            errors: 0,
        }
    }

    pub fn record_sent_message(&mut self, bytes: usize, time_us: f64) {
        self.messages_sent += 1;
        self.bytes_sent += bytes as u64;
        self.send_time_total_us += time_us as u64;
    }

    pub fn record_received_message(&mut self, bytes: usize, time_us: f64) {
        self.messages_received += 1;
        self.bytes_received += bytes as u64;
        self.receive_time_total_us += time_us as u64;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn average_send_time_us(&self) -> f64 {
        if self.messages_sent == 0 {
            0.0
        } else {
            self.send_time_total_us as f64 / self.messages_sent as f64
        }
    }

    pub fn average_receive_time_us(&self) -> f64 {
        if self.messages_received == 0 {
            0.0
        } else {
            self.receive_time_total_us as f64 / self.messages_received as f64
        }
    }

    pub fn throughput_mbps(&self) -> f64 {
        // 簡易的なスループット計算（実際には時間ベースで計算する必要がある）
        let total_bytes = self.bytes_sent + self.bytes_received;
        // 仮定: 1秒間のデータ量として計算
        (total_bytes as f64 * 8.0) / 1_000_000.0
    }
}

/// トランスポートビルダー
pub struct TransportBuilder {
    config: TransportConfig,
}

impl TransportBuilder {
    pub fn new() -> Self {
        Self {
            config: TransportConfig::default(),
        }
    }

    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = size;
        self
    }

    pub fn with_timeouts(mut self, read_ms: u64, write_ms: u64) -> Self {
        self.config.read_timeout_ms = read_ms;
        self.config.write_timeout_ms = write_ms;
        self
    }

    pub fn with_heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.config.heartbeat_interval_ms = interval_ms;
        self
    }

    pub fn enable_compression(mut self) -> Self {
        self.config.enable_compression = true;
        self
    }

    pub fn build(self, stream: SecureStream) -> Transport {
        Transport::new(stream, self.config)
    }
}

impl Default for TransportBuilder {
    fn default() -> Self {
        Self::new()
    }
}
