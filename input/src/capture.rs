//! 入力キャプチャ実装

use crate::KvmResult;
use soft_kvm_core::{InputEvent, Timer, PerformanceStats};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn, error};
use std::sync::Arc;

/// 入力キャプチャ設定
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub keyboard_capture: bool,
    pub mouse_capture: bool,
    pub capture_rate_hz: u32,
    pub buffer_size: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            keyboard_capture: true,
            mouse_capture: true,
            capture_rate_hz: 1000, // 1kHz
            buffer_size: 100,
        }
    }
}

/// 入力キャプチャー
pub struct InputCapturer {
    config: CaptureConfig,
    stats: Arc<std::sync::Mutex<PerformanceStats>>,
}

impl InputCapturer {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            stats: Arc::new(std::sync::Mutex::new(PerformanceStats::new())),
        }
    }

    /// 入力キャプチャを開始
    pub async fn start_capture(&self) -> KvmResult<InputCaptureStream> {
        info!("Starting input capture: keyboard={}, mouse={}, rate={}Hz",
              self.config.keyboard_capture, self.config.mouse_capture, self.config.capture_rate_hz);

        let (event_sender, event_receiver) = mpsc::channel(self.config.buffer_size);

        // プラットフォーム固有のキャプチャを開始
        #[cfg(target_os = "linux")]
        {
            crate::linux::start_capture(self.config.clone(), event_sender.clone()).await?;
        }

        #[cfg(target_os = "macos")]
        {
            crate::macos::start_capture(self.config.clone(), event_sender.clone()).await?;
        }

        #[cfg(target_os = "windows")]
        {
            crate::windows::start_capture(self.config.clone(), event_sender.clone()).await?;
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // テスト用のダミー実装
            self.start_dummy_capture(event_sender.clone()).await;
        }

        Ok(InputCaptureStream {
            receiver: event_receiver,
            stats: Arc::clone(&self.stats),
            config: self.config.clone(),
        })
    }

    /// テスト用のダミーキャプチャを開始
    async fn start_dummy_capture(&self, sender: mpsc::Sender<InputEvent>) {
        info!("Starting dummy input capture for testing");

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100)); // 10Hz
            let mut counter = 0;

            loop {
                interval.tick().await;

                // ダミー入力イベントを生成
                let event = match counter % 3 {
                    0 => InputEvent::MouseMove {
                        x: 100 + (counter % 100) as i32,
                        y: 100 + (counter % 100) as i32,
                        delta_x: 5,
                        delta_y: 3,
                    },
                    1 => InputEvent::MouseButton {
                        button: soft_kvm_core::MouseButton::Left,
                        pressed: (counter % 2) == 0,
                        x: 100,
                        y: 100,
                    },
                    _ => InputEvent::Keyboard {
                        keycode: 65, // 'A' key
                        pressed: (counter % 2) == 0,
                        modifiers: Default::default(),
                    },
                };

                counter += 1;

                if let Err(_) = sender.send(event).await {
                    debug!("Input capture stream closed");
                    break;
                }
            }
        });
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> PerformanceStats {
        self.stats.lock().unwrap().clone()
    }

    /// 設定を取得
    pub fn get_config(&self) -> &CaptureConfig {
        &self.config
    }
}

/// 入力キャプチャストリーム
pub struct InputCaptureStream {
    receiver: mpsc::Receiver<InputEvent>,
    stats: Arc<std::sync::Mutex<PerformanceStats>>,
    config: CaptureConfig,
}

impl InputCaptureStream {
    /// 次の入力イベントを取得
    pub async fn next_event(&mut self) -> Option<InputEvent> {
        let start_time = std::time::Instant::now();
        let event = self.receiver.recv().await?;

        // 統計更新
        let processing_time = start_time.elapsed().as_secs_f64() * 1000.0;
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record(processing_time);
        }

        Some(event)
    }

    /// ストリームを閉じる
    pub fn close(&mut self) {
        // 受信側をドロップすることでストリームを閉じる
        let _ = self.receiver.try_recv();
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> PerformanceStats {
        self.stats.lock().unwrap().clone()
    }

    /// 設定を取得
    pub fn get_config(&self) -> &CaptureConfig {
        &self.config
    }
}

/// プラットフォーム固有のキャプチャ実装

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use tokio::sync::mpsc::Sender;

    pub async fn start_capture(config: CaptureConfig, sender: Sender<InputEvent>) -> KvmResult<()> {
        info!("Starting Linux input capture");

        // TODO: evdevまたはlibinputを使用した実際の入力キャプチャ実装
        // 現在は簡易実装

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000 / config.capture_rate_hz as u64));
            let mut counter = 0;

            loop {
                interval.tick().await;

                // 簡易的な入力イベント生成
                if config.mouse_capture {
                    let mouse_event = InputEvent::MouseMove {
                        x: 500 + (counter % 200) as i32,
                        y: 300 + (counter % 200) as i32,
                        delta_x: (counter % 10) as i32 - 5,
                        delta_y: (counter % 8) as i32 - 4,
                    };

                    if let Err(_) = sender.send(mouse_event).await {
                        break;
                    }
                }

                if config.keyboard_capture && counter % 50 == 0 {
                    let key_event = InputEvent::Keyboard {
                        keycode: 65 + (counter % 26) as u32, // A-Z
                        pressed: (counter % 2) == 0,
                        modifiers: Default::default(),
                    };

                    if let Err(_) = sender.send(key_event).await {
                        break;
                    }
                }

                counter += 1;
            }
        });

        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;
    use tokio::sync::mpsc::Sender;

    pub async fn start_capture(config: CaptureConfig, sender: Sender<InputEvent>) -> KvmResult<()> {
        info!("Starting macOS input capture");

        // TODO: NSEventまたはCGEventを使用した実際の入力キャプチャ実装
        // 現在は簡易実装

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000 / config.capture_rate_hz as u64));
            let mut counter = 0;

            loop {
                interval.tick().await;

                if config.mouse_capture {
                    let mouse_event = InputEvent::MouseMove {
                        x: 800 + (counter % 300) as i32,
                        y: 600 + (counter % 300) as i32,
                        delta_x: (counter % 12) as i32 - 6,
                        delta_y: (counter % 10) as i32 - 5,
                    };

                    if let Err(_) = sender.send(mouse_event).await {
                        break;
                    }
                }

                if config.keyboard_capture && counter % 30 == 0 {
                    let key_event = InputEvent::Keyboard {
                        keycode: 65 + (counter % 26) as u32,
                        pressed: (counter % 2) == 0,
                        modifiers: soft_kvm_core::KeyModifiers {
                            cmd: (counter % 100) < 10, // 時々Commandキー
                            ..Default::default()
                        },
                    };

                    if let Err(_) = sender.send(key_event).await {
                        break;
                    }
                }

                counter += 1;
            }
        });

        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use tokio::sync::mpsc::Sender;

    pub async fn start_capture(config: CaptureConfig, sender: Sender<InputEvent>) -> KvmResult<()> {
        info!("Starting Windows input capture");

        // TODO: Raw Input APIまたはWindows Hookを使用した実際の入力キャプチャ実装
        // 現在は簡易実装

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(1000 / config.capture_rate_hz as u64));
            let mut counter = 0;

            loop {
                interval.tick().await;

                if config.mouse_capture {
                    let mouse_event = InputEvent::MouseMove {
                        x: 1024 + (counter % 400) as i32,
                        y: 768 + (counter % 400) as i32,
                        delta_x: (counter % 14) as i32 - 7,
                        delta_y: (counter % 12) as i32 - 6,
                    };

                    if let Err(_) = sender.send(mouse_event).await {
                        break;
                    }
                }

                if config.keyboard_capture && counter % 40 == 0 {
                    let key_event = InputEvent::Keyboard {
                        keycode: 65 + (counter % 26) as u32,
                        pressed: (counter % 2) == 0,
                        modifiers: soft_kvm_core::KeyModifiers {
                            ctrl: (counter % 150) < 15,
                            ..Default::default()
                        },
                    };

                    if let Err(_) = sender.send(key_event).await {
                        break;
                    }
                }

                counter += 1;
            }
        });

        Ok(())
    }
}
