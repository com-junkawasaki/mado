//! ビデオエンコーダー実装

use crate::{CapturedFrame, KvmResult, CaptureFormat};
use soft_kvm_core::{Resolution, VideoQuality, Timer, PerformanceStats};
use openh264::encoder::{Encoder, EncoderConfig};
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};
use std::sync::Arc;

/// エンコード設定
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub quality: VideoQuality,
    pub keyframe_interval: u32,
    pub threads: usize,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            quality: VideoQuality::balanced(),
            keyframe_interval: 30, // 1秒に1回
            threads: 4,
        }
    }
}

/// エンコードされたフレーム
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub frame_type: FrameType,
    pub original_width: u32,
    pub original_height: u32,
    pub timestamp: u64,
    pub sequence_number: u64,
}

/// フレームタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    KeyFrame,
    PFrame,
    BFrame,
}

/// H.264ビデオエンコーダー
pub struct H264Encoder {
    encoder: Encoder,
    config: EncoderConfig,
    stats: Arc<std::sync::Mutex<PerformanceStats>>,
    sequence_counter: Arc<std::sync::Mutex<u64>>,
    frame_counter: Arc<std::sync::Mutex<u64>>,
}

impl H264Encoder {
    pub fn new(config: EncoderConfig) -> KvmResult<Self> {
        // OpenH264エンコーダーの設定
        let mut encoder_config = EncoderConfig::new();
        encoder_config.width = config.quality.fps as i32; // 一時的にfpsを使用
        encoder_config.height = config.quality.bitrate_mbps as i32; // 一時的にbitrateを使用
        encoder_config.bitrate = (config.quality.bitrate_mbps * 1_000_000) as i32;
        encoder_config.framerate = config.quality.fps as f32;
        encoder_config.keyframe_interval = config.keyframe_interval;

        let encoder = Encoder::new(encoder_config)
            .map_err(|e| soft_kvm_core::KvmError::Video(format!("Failed to create encoder: {:?}", e)))?;

        Ok(Self {
            encoder,
            config,
            stats: Arc::new(std::sync::Mutex::new(PerformanceStats::new())),
            sequence_counter: Arc::new(std::sync::Mutex::new(0)),
            frame_counter: Arc::new(std::sync::Mutex::new(0)),
        })
    }

    /// フレームをエンコード
    pub fn encode_frame(&mut self, frame: &CapturedFrame) -> KvmResult<EncodedFrame> {
        let start_time = std::time::Instant::now();

        // RGBからYUV420に変換（OpenH264が必要とする形式）
        let yuv_data = self.convert_to_yuv420(frame)?;

        // エンコード
        let encoded_data = self.encoder.encode(&yuv_data)
            .map_err(|e| soft_kvm_core::KvmError::Video(format!("Encoding failed: {:?}", e)))?;

        let encode_time = start_time.elapsed().as_secs_f64() * 1000.0;

        // 統計更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record(encode_time);
        }

        // フレームタイプを判定
        let frame_type = self.determine_frame_type(&encoded_data);

        let sequence_number = {
            let mut counter = self.sequence_counter.lock().unwrap();
            *counter += 1;
            *counter
        };

        debug!("Encoded frame: size={} bytes, time={:.2}ms, type={:?}",
               encoded_data.len(), encode_time, frame_type);

        Ok(EncodedFrame {
            data: encoded_data,
            frame_type,
            original_width: frame.width,
            original_height: frame.height,
            timestamp: frame.timestamp,
            sequence_number,
        })
    }

    /// RGBからYUV420に変換
    fn convert_to_yuv420(&self, frame: &CapturedFrame) -> KvmResult<Vec<u8>> {
        match frame.format {
            CaptureFormat::Rgb => {
                // RGB to YUV420変換
                let width = frame.width as usize;
                let height = frame.height as usize;
                let mut yuv = vec![0u8; width * height * 3 / 2]; // YUV420のサイズ

                // 簡易的なRGB to YUV変換
                for i in 0..(width * height) {
                    let r = frame.data[i * 3] as f32;
                    let g = frame.data[i * 3 + 1] as f32;
                    let b = frame.data[i * 3 + 2] as f32;

                    // YUV変換公式
                    let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                    let u = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0) as u8;
                    let v = (0.500 * r - 0.419 * g - 0.081 * b + 128.0) as u8;

                    yuv[i] = y;

                    // UとVは4:2:0サブサンプリング
                    if i % 2 == 0 {
                        let uv_index = width * height + (i / 2);
                        yuv[uv_index] = u;
                        yuv[uv_index + (width * height / 4)] = v;
                    }
                }

                Ok(yuv)
            }
            CaptureFormat::Bgra => {
                // BGRA to RGB変換後にYUV変換
                warn!("BGRA format not fully implemented, using RGB conversion");
                self.convert_to_yuv420(&CapturedFrame {
                    format: CaptureFormat::Rgb,
                    ..frame.clone()
                })
            }
            CaptureFormat::Yuv420 => {
                // 既にYUV420
                Ok(frame.data.clone())
            }
        }
    }

    /// フレームタイプを判定
    fn determine_frame_type(&self, encoded_data: &[u8]) -> FrameType {
        // H.264 NAL unitの先頭バイトで判定
        if encoded_data.len() < 5 {
            return FrameType::PFrame;
        }

        // NAL unit type (下位5ビット)
        let nal_type = encoded_data[4] & 0x1F;

        match nal_type {
            1 | 2 | 3 | 4 => FrameType::PFrame, // P/Bフレーム
            5 => FrameType::KeyFrame,           // IDRピクチャ (キーフレーム)
            6 => FrameType::PFrame,             // SEI
            7 => FrameType::KeyFrame,           // SPS
            8 => FrameType::KeyFrame,           // PPS
            _ => FrameType::PFrame,
        }
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> PerformanceStats {
        self.stats.lock().unwrap().clone()
    }

    /// エンコーダーを閉じる
    pub fn close(&mut self) {
        // エンコーダーのクリーンアップ
        info!("H.264 encoder closed");
    }
}

impl Drop for H264Encoder {
    fn drop(&mut self) {
        self.close();
    }
}

/// ビデオエンコードストリーム
pub struct VideoEncodeStream {
    encoder: H264Encoder,
    receiver: mpsc::Receiver<CapturedFrame>,
    sender: mpsc::Sender<EncodedFrame>,
}

impl VideoEncodeStream {
    pub fn new(
        encoder: H264Encoder,
        receiver: mpsc::Receiver<CapturedFrame>,
        sender: mpsc::Sender<EncodedFrame>,
    ) -> Self {
        Self {
            encoder,
            receiver,
            sender,
        }
    }

    /// エンコード処理を開始
    pub async fn start_encoding(mut self) -> KvmResult<()> {
        info!("Starting video encoding stream");

        tokio::spawn(async move {
            loop {
                match self.receiver.recv().await {
                    Some(frame) => {
                        match self.encoder.encode_frame(&frame) {
                            Ok(encoded_frame) => {
                                if let Err(_) = self.sender.send(encoded_frame).await {
                                    debug!("Encode stream closed");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Frame encoding failed: {}", e);
                                // エンコード失敗時はスキップ
                            }
                        }
                    }
                    None => {
                        debug!("Capture stream ended");
                        break;
                    }
                }
            }

            info!("Video encoding stream ended");
        });

        Ok(())
    }
}

/// ビデオ処理パイプライン
pub struct VideoPipeline {
    capturer: VideoCapturer,
    encoder: H264Encoder,
}

impl VideoPipeline {
    pub fn new(capture_config: crate::CaptureConfig, encode_config: EncoderConfig) -> KvmResult<Self> {
        let capturer = VideoCapturer::new(capture_config);
        let encoder = H264Encoder::new(encode_config)?;

        Ok(Self {
            capturer,
            encoder,
        })
    }

    /// パイプラインを開始
    pub async fn start_pipeline(&mut self) -> KvmResult<mpsc::Receiver<EncodedFrame>> {
        let (encoded_sender, encoded_receiver) = mpsc::channel(10);

        // キャプチャを開始
        let mut capture_stream = self.capturer.start_capture().await?;

        // エンコードストリームを作成
        let (capture_sender, capture_receiver) = mpsc::channel(10);
        let encode_stream = VideoEncodeStream::new(
            self.encoder.clone(),
            capture_receiver,
            encoded_sender,
        );

        // エンコードを開始
        encode_stream.start_encoding().await?;

        // キャプチャからエンコードへのデータ転送を開始
        tokio::spawn(async move {
            while let Some(frame) = capture_stream.next_frame().await {
                if let Err(_) = capture_sender.send(frame).await {
                    break;
                }
            }
        });

        Ok(encoded_receiver)
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> (PerformanceStats, PerformanceStats) {
        (self.capturer.get_stats(), self.encoder.get_stats())
    }
}

// H264EncoderのClone実装
impl Clone for H264Encoder {
    fn clone(&self) -> Self {
        // 注意: エンコーダーはスレッドセーフではないので、
        // 実際の実装では各スレッドで別々のエンコーダーを作成する必要がある
        Self {
            encoder: todo!("Encoder cloning not implemented"),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            sequence_counter: Arc::clone(&self.sequence_counter),
            frame_counter: Arc::clone(&self.frame_counter),
        }
    }
}
