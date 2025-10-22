//! 共通型定義

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

/// サービス識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Uuid);

impl ServiceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ServiceId {
    fn default() -> Self {
        Self::new()
    }
}

/// ネットワークアドレス
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAddress {
    pub ip: IpAddr,
    pub port: u16,
}

impl NetworkAddress {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
    }

    pub fn localhost(port: u16) -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port,
        }
    }
}

/// ビデオ解像度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn hd() -> Self {
        Self::new(1920, 1080)
    }

    pub fn fhd() -> Self {
        Self::new(1920, 1080)
    }

    pub fn qhd() -> Self {
        Self::new(2560, 1440)
    }

    pub fn uhd() -> Self {
        Self::new(3840, 2160)
    }
}

/// ビデオ品質設定
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoQuality {
    pub fps: u32,
    pub bitrate_mbps: u32,
    pub compression_quality: f32, // 0.0-1.0
}

impl VideoQuality {
    pub fn low_latency() -> Self {
        Self {
            fps: 60,
            bitrate_mbps: 50,
            compression_quality: 0.8,
        }
    }

    pub fn balanced() -> Self {
        Self {
            fps: 30,
            bitrate_mbps: 25,
            compression_quality: 0.9,
        }
    }

    pub fn high_quality() -> Self {
        Self {
            fps: 30,
            bitrate_mbps: 100,
            compression_quality: 0.95,
        }
    }
}

/// 入力イベント種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Keyboard {
        keycode: u32,
        pressed: bool,
        modifiers: KeyModifiers,
    },
    MouseButton {
        button: MouseButton,
        pressed: bool,
        x: i32,
        y: i32,
    },
    MouseMove {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    MouseWheel {
        delta_x: i32,
        delta_y: i32,
    },
}

/// キーボード修飾キー
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool, // Windows/Super/Command key
}

impl KeyModifiers {
    pub fn none() -> Self {
        Self::default()
    }
}

/// マウスボタン
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Button4,
    Button5,
}

/// 接続状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Active,
    Error,
}

/// メトリクスデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub video_latency_ms: f64,
    pub input_latency_ms: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub network_bytes_per_sec: u64,
}
