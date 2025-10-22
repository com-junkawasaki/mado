//! ビデオストリーミング実装

use crate::{EncodedFrame, VideoPipeline, CaptureConfig, EncoderConfig, KvmResult};
use soft_kvm_core::KvmResult as CoreResult;
use soft_kvm_protocol::{VideoFrameMessage, VideoConfigMessage, MessageBuilder, Transport, SessionManager, KvmSession};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tracing::{debug, info, warn, error};
use std::sync::Arc;

/// ストリーミング設定
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub capture_config: CaptureConfig,
    pub encode_config: EncoderConfig,
    pub target_fps: u32,
    pub max_buffer_frames: usize,
    pub adaptive_quality: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            capture_config: CaptureConfig::default(),
            encode_config: EncoderConfig::default(),
            target_fps: 30,
            max_buffer_frames: 10,
            adaptive_quality: true,
        }
    }
}

/// ビデオストリーマー
pub struct VideoStreamer {
    config: StreamingConfig,
    pipeline: Option<VideoPipeline>,
    session_manager: Arc<SessionManager>,
    message_builder: Arc<std::sync::Mutex<MessageBuilder>>,
}

impl VideoStreamer {
    pub fn new(config: StreamingConfig, session_manager: Arc<SessionManager>) -> Self {
        Self {
            config,
            pipeline: None,
            session_manager,
            message_builder: Arc::new(std::sync::Mutex::new(MessageBuilder::new())),
        }
    }

    /// ストリーミングを開始
    pub async fn start_streaming(&mut self) -> KvmResult<()> {
        info!("Starting video streaming");

        // ビデオパイプラインを作成
        let mut pipeline = VideoPipeline::new(
            self.config.capture_config.clone(),
            self.config.encode_config.clone(),
        )?;

        // エンコードストリームを取得
        let encoded_receiver = pipeline.start_pipeline().await?;
        self.pipeline = Some(pipeline);

        // ストリーミングタスクを開始
        self.start_streaming_task(encoded_receiver).await?;

        Ok(())
    }

    /// ストリーミングタスクを開始
    async fn start_streaming_task(&self, mut encoded_receiver: mpsc::Receiver<EncodedFrame>) -> KvmResult<()> {
        let session_manager = Arc::clone(&self.session_manager);
        let message_builder = Arc::clone(&self.message_builder);
        let target_frame_interval = Duration::from_millis(1000 / self.config.target_fps as u64);

        tokio::spawn(async move {
            let mut last_frame_time = tokio::time::Instant::now();
            let mut frame_buffer = Vec::new();

            loop {
                // フレームレート制御
                let elapsed = last_frame_time.elapsed();
                if elapsed < target_frame_interval {
                    tokio::time::sleep(target_frame_interval - elapsed).await;
                }
                last_frame_time = tokio::time::Instant::now();

                // エンコードされたフレームを取得（タイムアウト付き）
                let encoded_frame = match timeout(Duration::from_millis(100), encoded_receiver.recv()).await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        info!("Video encoding stream ended");
                        break;
                    }
                    Err(_) => {
                        // タイムアウト時は前回のフレームを再送信（オプション）
                        if !frame_buffer.is_empty() {
                            if let Some(last_frame) = frame_buffer.last() {
                                Self::broadcast_frame(&session_manager, &message_builder, last_frame).await;
                            }
                        }
                        continue;
                    }
                };

                // バッファ管理
                frame_buffer.push(encoded_frame.clone());
                if frame_buffer.len() > 10 { // 最大バッファサイズ
                    frame_buffer.remove(0);
                }

                // 全セッションにフレームをブロードキャスト
                Self::broadcast_frame(&session_manager, &message_builder, &encoded_frame).await;
            }
        });

        Ok(())
    }

    /// フレームを全セッションにブロードキャスト
    async fn broadcast_frame(
        session_manager: &SessionManager,
        message_builder: &std::sync::Mutex<MessageBuilder>,
        frame: &EncodedFrame,
    ) {
        let sessions = session_manager.get_all_sessions().await;

        if sessions.is_empty() {
            return;
        }

        // VideoFrameメッセージを作成
        let video_frame = VideoFrameMessage {
            frame_number: frame.sequence_number as u32,
            timestamp: frame.timestamp,
            encoded_data: frame.data.clone(),
            width: frame.original_width,
            height: frame.original_height,
            codec: "h264".to_string(),
        };

        let message = {
            let mut builder = message_builder.lock().unwrap();
            builder.build_video_frame(video_frame)
        };

        // 全アクティブセッションに送信
        for session in sessions {
            if session.is_active().await {
                if let Err(e) = session.send_message(message.clone()).await {
                    warn!("Failed to send video frame to session {}: {}", session.get_info().await.session_id, e);
                } else {
                    debug!("Sent video frame {} to session {}", frame.sequence_number, session.get_info().await.session_id);
                }
            }
        }
    }

    /// ビデオ設定を送信
    pub async fn send_video_config(&self, session: &KvmSession) -> KvmResult<()> {
        let config_msg = VideoConfigMessage {
            resolution: Some(self.config.capture_config.resolution.into()),
            quality: Some(self.config.encode_config.quality.into()),
            codec: "h264".to_string(),
        };

        let payload = config_msg.encode_to_vec();
        let message = {
            let mut builder = self.message_builder.lock().unwrap();
            soft_kvm_protocol::ProtocolMessage::new(
                soft_kvm_protocol::MessageType::VideoConfig,
                payload
            ).with_sequence(builder.next_sequence())
        };

        session.send_message(message).await?;
        info!("Sent video config to session {}", session.get_info().await.session_id);

        Ok(())
    }

    /// ストリーミングを停止
    pub async fn stop_streaming(&mut self) -> KvmResult<()> {
        if let Some(pipeline) = self.pipeline.take() {
            // パイプラインのクリーンアップ
            drop(pipeline);
        }

        info!("Video streaming stopped");
        Ok(())
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> Option<(soft_kvm_core::PerformanceStats, soft_kvm_core::PerformanceStats)> {
        self.pipeline.as_ref().map(|p| p.get_stats())
    }

    /// 設定を取得
    pub fn get_config(&self) -> &StreamingConfig {
        &self.config
    }

    /// アクティブなストリームがあるかチェック
    pub fn is_streaming(&self) -> bool {
        self.pipeline.is_some()
    }
}

/// 品質適応コントローラー
pub struct QualityController {
    current_quality: VideoQuality,
    target_fps: u32,
    adaptive_mode: bool,
}

impl QualityController {
    pub fn new(initial_quality: VideoQuality, target_fps: u32, adaptive_mode: bool) -> Self {
        Self {
            current_quality: initial_quality,
            target_fps,
            adaptive_mode,
        }
    }

    /// ネットワーク状態に基づいて品質を適応
    pub fn adapt_quality(&mut self, network_latency_ms: f64, packet_loss_rate: f64) {
        if !self.adaptive_mode {
            return;
        }

        let new_quality = if network_latency_ms > 100.0 || packet_loss_rate > 0.05 {
            // 高遅延または高パケットロス時は品質を下げる
            VideoQuality {
                fps: self.target_fps.min(15),
                bitrate_mbps: (self.current_quality.bitrate_mbps / 2).max(5),
                compression_quality: 0.7,
            }
        } else if network_latency_ms < 20.0 && packet_loss_rate < 0.01 {
            // 良好なネットワーク時は品質を上げる
            VideoQuality {
                fps: self.target_fps,
                bitrate_mbps: (self.current_quality.bitrate_mbps + 5).min(50),
                compression_quality: 0.9,
            }
        } else {
            self.current_quality.clone()
        };

        if new_quality != self.current_quality {
            info!("Adapting video quality: fps={}, bitrate={}Mbps",
                  new_quality.fps, new_quality.bitrate_mbps);
            self.current_quality = new_quality;
        }
    }

    /// 現在の品質を取得
    pub fn get_current_quality(&self) -> &VideoQuality {
        &self.current_quality
    }
}

/// ビデオストリーミングマネージャー
pub struct VideoStreamingManager {
    streamer: Option<VideoStreamer>,
    quality_controller: QualityController,
    sessions: Arc<SessionManager>,
}

impl VideoStreamingManager {
    pub fn new(sessions: Arc<SessionManager>) -> Self {
        Self {
            streamer: None,
            quality_controller: QualityController::new(
                VideoQuality::balanced(),
                30,
                true,
            ),
            sessions,
        }
    }

    /// ストリーミングを開始
    pub async fn start_streaming(&mut self, config: StreamingConfig) -> KvmResult<()> {
        let streamer = VideoStreamer::new(config, Arc::clone(&self.sessions));
        self.streamer = Some(streamer);

        if let Some(ref mut streamer) = self.streamer {
            streamer.start_streaming().await?;
        }

        info!("Video streaming manager started");
        Ok(())
    }

    /// ストリーミングを停止
    pub async fn stop_streaming(&mut self) -> KvmResult<()> {
        if let Some(mut streamer) = self.streamer.take() {
            streamer.stop_streaming().await?;
        }

        info!("Video streaming manager stopped");
        Ok(())
    }

    /// 品質適応を実行
    pub fn adapt_quality(&mut self, network_stats: &NetworkStats) {
        self.quality_controller.adapt_quality(
            network_stats.average_latency_ms,
            network_stats.packet_loss_rate,
        );
    }

    /// セッションにビデオ設定を送信
    pub async fn configure_session(&self, session: &KvmSession) -> KvmResult<()> {
        if let Some(ref streamer) = self.streamer {
            streamer.send_video_config(session).await?;
        }
        Ok(())
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> Option<(soft_kvm_core::PerformanceStats, soft_kvm_core::PerformanceStats)> {
        self.streamer.as_ref()?.get_stats()
    }
}

/// ネットワーク統計
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub average_latency_ms: f64,
    pub packet_loss_rate: f64,
    pub bandwidth_mbps: f64,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self {
            average_latency_ms: 0.0,
            packet_loss_rate: 0.0,
            bandwidth_mbps: 0.0,
        }
    }
}
