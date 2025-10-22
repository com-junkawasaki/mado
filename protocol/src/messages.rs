//! プロトコルメッセージ定義

use prost::Message;
use serde::{Deserialize, Serialize};
use soft_kvm_core::{ServiceId, NetworkAddress, Resolution, VideoQuality, InputEvent};
use std::time::{SystemTime, UNIX_EPOCH};

/// メッセージタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    // 制御メッセージ
    Hello = 0x01,
    Welcome = 0x02,
    Heartbeat = 0x03,
    Goodbye = 0x04,

    // ビデオメッセージ
    VideoFrame = 0x10,
    VideoConfig = 0x11,
    VideoAck = 0x12,

    // 入力メッセージ
    InputEvent = 0x20,
    InputAck = 0x21,

    // エラーメッセージ
    Error = 0xF0,
}

/// プロトコルメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub message_type: MessageType,
    pub sequence_number: u32,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl ProtocolMessage {
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            sequence_number: 0, // シーケンス番号は送信時に設定
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            payload,
        }
    }

    pub fn with_sequence(mut self, seq: u32) -> Self {
        self.sequence_number = seq;
        self
    }

    /// メッセージをバイト列にシリアライズ
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // ヘッダー: メッセージタイプ(1) + シーケンス番号(4) + タイムスタンプ(8) + ペイロード長(4)
        buf.push(self.message_type as u8);
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// バイト列からメッセージをデシリアライズ
    pub fn from_bytes(data: &[u8]) -> Result<Self, ProtocolError> {
        if data.len() < 13 {
            return Err(ProtocolError::InvalidMessage("Message too short".to_string()));
        }

        let message_type = match data[0] {
            0x01 => MessageType::Hello,
            0x02 => MessageType::Welcome,
            0x03 => MessageType::Heartbeat,
            0x04 => MessageType::Goodbye,
            0x10 => MessageType::VideoFrame,
            0x11 => MessageType::VideoConfig,
            0x12 => MessageType::VideoAck,
            0x20 => MessageType::InputEvent,
            0x21 => MessageType::InputAck,
            0xF0 => MessageType::Error,
            _ => return Err(ProtocolError::InvalidMessage("Unknown message type".to_string())),
        };

        let sequence_number = u32::from_le_bytes(data[1..5].try_into().unwrap());
        let timestamp = u64::from_le_bytes(data[5..13].try_into().unwrap());
        let payload_len = u32::from_le_bytes(data[13..17].try_into().unwrap()) as usize;

        if data.len() < 17 + payload_len {
            return Err(ProtocolError::InvalidMessage("Incomplete payload".to_string()));
        }

        let payload = data[17..17 + payload_len].to_vec();

        Ok(Self {
            message_type,
            sequence_number,
            timestamp,
            payload,
        })
    }
}

/// Helloメッセージ（接続開始）
#[derive(Message, Clone)]
pub struct HelloMessage {
    #[prost(string, tag = "1")]
    pub protocol_version: String,
    #[prost(string, tag = "2")]
    pub client_name: String,
    #[prost(message, optional, tag = "3")]
    pub capabilities: Option<ClientCapabilities>,
}

/// Welcomeメッセージ（接続承認）
#[derive(Message, Clone)]
pub struct WelcomeMessage {
    #[prost(string, tag = "1")]
    pub server_name: String,
    #[prost(message, optional, tag = "2")]
    pub server_capabilities: Option<ServerCapabilities>,
    #[prost(uint32, tag = "3")]
    pub session_id: u32,
}

/// クライアント機能
#[derive(Message, Clone)]
pub struct ClientCapabilities {
    #[prost(bool, tag = "1")]
    pub supports_video: bool,
    #[prost(bool, tag = "2")]
    pub supports_input: bool,
    #[prost(repeated message, tag = "3")]
    pub supported_resolutions: Vec<ResolutionMessage>,
    #[prost(repeated message, tag = "4")]
    pub supported_qualities: Vec<VideoQualityMessage>,
}

/// サーバー機能
#[derive(Message, Clone)]
pub struct ServerCapabilities {
    #[prost(bool, tag = "1")]
    pub supports_video: bool,
    #[prost(bool, tag = "2")]
    pub supports_input: bool,
    #[prost(message, optional, tag = "3")]
    pub current_resolution: Option<ResolutionMessage>,
    #[prost(message, optional, tag = "4")]
    pub current_quality: Option<VideoQualityMessage>,
    #[prost(uint32, tag = "5")]
    pub max_clients: u32,
}

/// 解像度メッセージ
#[derive(Message, Clone)]
pub struct ResolutionMessage {
    #[prost(uint32, tag = "1")]
    pub width: u32,
    #[prost(uint32, tag = "2")]
    pub height: u32,
}

impl From<Resolution> for ResolutionMessage {
    fn from(res: Resolution) -> Self {
        Self {
            width: res.width,
            height: res.height,
        }
    }
}

impl From<ResolutionMessage> for Resolution {
    fn from(msg: ResolutionMessage) -> Self {
        Self::new(msg.width, msg.height)
    }
}

/// ビデオ品質メッセージ
#[derive(Message, Clone)]
pub struct VideoQualityMessage {
    #[prost(uint32, tag = "1")]
    pub fps: u32,
    #[prost(uint32, tag = "2")]
    pub bitrate_mbps: u32,
    #[prost(float, tag = "3")]
    pub compression_quality: f32,
}

impl From<VideoQuality> for VideoQualityMessage {
    fn from(quality: VideoQuality) -> Self {
        Self {
            fps: quality.fps,
            bitrate_mbps: quality.bitrate_mbps,
            compression_quality: quality.compression_quality,
        }
    }
}

impl From<VideoQualityMessage> for VideoQuality {
    fn from(msg: VideoQualityMessage) -> Self {
        Self {
            fps: msg.fps,
            bitrate_mbps: msg.bitrate_mbps,
            compression_quality: msg.compression_quality,
        }
    }
}

/// ビデオフレームメッセージ
#[derive(Message, Clone)]
pub struct VideoFrameMessage {
    #[prost(uint32, tag = "1")]
    pub frame_number: u32,
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
    #[prost(bytes, tag = "3")]
    pub encoded_data: Vec<u8>,
    #[prost(uint32, tag = "4")]
    pub width: u32,
    #[prost(uint32, tag = "5")]
    pub height: u32,
    #[prost(string, tag = "6")]
    pub codec: String,
}

/// ビデオ設定メッセージ
#[derive(Message, Clone)]
pub struct VideoConfigMessage {
    #[prost(message, optional, tag = "1")]
    pub resolution: Option<ResolutionMessage>,
    #[prost(message, optional, tag = "2")]
    pub quality: Option<VideoQualityMessage>,
    #[prost(string, tag = "3")]
    pub codec: String,
}

/// 入力イベントメッセージ
#[derive(Message, Clone)]
pub struct InputEventMessage {
    #[prost(uint64, tag = "1")]
    pub timestamp: u64,
    #[prost(oneof = "input_event", tags = "2,3,4,5")]
    pub event: Option<input_event::Event>,
}

pub mod input_event {
    use prost::Oneof;

    #[derive(Clone, Oneof)]
    pub enum Event {
        #[prost(message, tag = "2")]
        Keyboard(KeyboardEvent),
        #[prost(message, tag = "3")]
        MouseButton(MouseButtonEvent),
        #[prost(message, tag = "4")]
        MouseMove(MouseMoveEvent),
        #[prost(message, tag = "5")]
        MouseWheel(MouseWheelEvent),
    }

    #[derive(Message, Clone)]
    pub struct KeyboardEvent {
        #[prost(uint32, tag = "1")]
        pub keycode: u32,
        #[prost(bool, tag = "2")]
        pub pressed: bool,
        #[prost(uint32, tag = "3")]
        pub modifiers: u32,
    }

    #[derive(Message, Clone)]
    pub struct MouseButtonEvent {
        #[prost(uint32, tag = "1")]
        pub button: u32,
        #[prost(bool, tag = "2")]
        pub pressed: bool,
        #[prost(int32, tag = "3")]
        pub x: i32,
        #[prost(int32, tag = "4")]
        pub y: i32,
    }

    #[derive(Message, Clone)]
    pub struct MouseMoveEvent {
        #[prost(int32, tag = "1")]
        pub x: i32,
        #[prost(int32, tag = "2")]
        pub y: i32,
        #[prost(int32, tag = "3")]
        pub delta_x: i32,
        #[prost(int32, tag = "4")]
        pub delta_y: i32,
    }

    #[derive(Message, Clone)]
    pub struct MouseWheelEvent {
        #[prost(int32, tag = "1")]
        pub delta_x: i32,
        #[prost(int32, tag = "2")]
        pub delta_y: i32,
    }
}

impl From<InputEvent> for InputEventMessage {
    fn from(event: InputEvent) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let event_data = match event {
            InputEvent::Keyboard { keycode, pressed, modifiers } => {
                input_event::Event::Keyboard(input_event::KeyboardEvent {
                    keycode,
                    pressed,
                    modifiers: modifiers.bits(),
                })
            }
            InputEvent::MouseButton { button, pressed, x, y } => {
                input_event::Event::MouseButton(input_event::MouseButtonEvent {
                    button: button as u32,
                    pressed,
                    x,
                    y,
                })
            }
            InputEvent::MouseMove { x, y, delta_x, delta_y } => {
                input_event::Event::MouseMove(input_event::MouseMoveEvent {
                    x,
                    y,
                    delta_x,
                    delta_y,
                })
            }
            InputEvent::MouseWheel { delta_x, delta_y } => {
                input_event::Event::MouseWheel(input_event::MouseWheelEvent {
                    delta_x,
                    delta_y,
                })
            }
        };

        Self {
            timestamp,
            event: Some(event_data),
        }
    }
}

/// エラーメッセージ
#[derive(Message, Clone)]
pub struct ErrorMessage {
    #[prost(uint32, tag = "1")]
    pub error_code: u32,
    #[prost(string, tag = "2")]
    pub message: String,
    #[prost(bool, tag = "3")]
    pub fatal: bool,
}

/// プロトコルエラー
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] prost::EncodeError),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] prost::DecodeError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported message type")]
    UnsupportedMessageType,

    #[error("Protocol version mismatch")]
    VersionMismatch,
}

/// メッセージビルダー
pub struct MessageBuilder {
    sequence_counter: u32,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self {
            sequence_counter: 0,
        }
    }

    pub fn next_sequence(&mut self) -> u32 {
        self.sequence_counter += 1;
        self.sequence_counter
    }

    pub fn build_hello(&mut self, client_name: &str) -> ProtocolMessage {
        let hello = HelloMessage {
            protocol_version: super::PROTOCOL_VERSION.to_string(),
            client_name: client_name.to_string(),
            capabilities: Some(ClientCapabilities {
                supports_video: true,
                supports_input: true,
                supported_resolutions: vec![
                    Resolution::hd().into(),
                    Resolution::fhd().into(),
                    Resolution::qhd().into(),
                ],
                supported_qualities: vec![
                    VideoQuality::low_latency().into(),
                    VideoQuality::balanced().into(),
                ],
            }),
        };

        let payload = hello.encode_to_vec();
        ProtocolMessage::new(MessageType::Hello, payload)
            .with_sequence(self.next_sequence())
    }

    pub fn build_welcome(&mut self, server_name: &str, session_id: u32) -> ProtocolMessage {
        let welcome = WelcomeMessage {
            server_name: server_name.to_string(),
            server_capabilities: Some(ServerCapabilities {
                supports_video: true,
                supports_input: true,
                current_resolution: Some(Resolution::fhd().into()),
                current_quality: Some(VideoQuality::balanced().into()),
                max_clients: 1,
            }),
            session_id,
        };

        let payload = welcome.encode_to_vec();
        ProtocolMessage::new(MessageType::Welcome, payload)
            .with_sequence(self.next_sequence())
    }

    pub fn build_heartbeat(&mut self) -> ProtocolMessage {
        ProtocolMessage::new(MessageType::Heartbeat, vec![])
            .with_sequence(self.next_sequence())
    }

    pub fn build_video_frame(&mut self, frame: VideoFrameMessage) -> ProtocolMessage {
        let payload = frame.encode_to_vec();
        ProtocolMessage::new(MessageType::VideoFrame, payload)
            .with_sequence(self.next_sequence())
    }

    pub fn build_input_event(&mut self, event: InputEvent) -> ProtocolMessage {
        let input_msg: InputEventMessage = event.into();
        let payload = input_msg.encode_to_vec();
        ProtocolMessage::new(MessageType::InputEvent, payload)
            .with_sequence(self.next_sequence())
    }

    pub fn build_error(&mut self, error_code: u32, message: &str, fatal: bool) -> ProtocolMessage {
        let error_msg = ErrorMessage {
            error_code,
            message: message.to_string(),
            fatal,
        };

        let payload = error_msg.encode_to_vec();
        ProtocolMessage::new(MessageType::Error, payload)
            .with_sequence(self.next_sequence())
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}
