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

//! Protocol message definitions

use serde::{Deserialize, Serialize};
use soft_kvm_core::*;

/// Message types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    // Control messages
    Hello,
    Welcome,
    Goodbye,
    Heartbeat,
    Error,

    // Authentication
    AuthRequest,
    AuthResponse,

    // KVM control
    VideoStart,
    VideoStop,
    VideoFrame,
    InputEvent,
    ClipboardData,

    // Service discovery
    ServiceAnnouncement,
    ServiceQuery,
    ServiceResponse,

    // Monitoring
    MetricsRequest,
    MetricsResponse,
    Ping,
    Pong,
}

/// Protocol message header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub message_type: MessageType,
    pub message_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: Option<String>,
    pub compression: bool,
    pub payload_size: usize,
}

/// Protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}

/// Message payload types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    // Control payloads
    Hello(HelloPayload),
    Welcome(WelcomePayload),
    Goodbye(GoodbyePayload),
    Heartbeat(HeartbeatPayload),
    Error(ErrorPayload),

    // Auth payloads
    AuthRequest(AuthRequestPayload),
    AuthResponse(AuthResponsePayload),

    // KVM payloads
    VideoStart(VideoStartPayload),
    VideoStop,
    VideoFrame(VideoFramePayload),
    InputEvent(InputEventPayload),
    ClipboardData(ClipboardPayload),

    // Discovery payloads
    ServiceAnnouncement(ServiceAnnouncementPayload),
    ServiceQuery(ServiceQueryPayload),
    ServiceResponse(ServiceResponsePayload),

    // Monitoring payloads
    MetricsRequest,
    MetricsResponse(MetricsPayload),
    Ping,
    Pong,
}

/// Hello message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloPayload {
    pub protocol_version: String,
    pub client_info: ClientInfo,
    pub capabilities: Vec<String>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: String,
    pub client_name: String,
    pub platform: String,
    pub version: String,
}

/// Welcome message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomePayload {
    pub server_info: ServerInfo,
    pub session_id: String,
    pub negotiated_capabilities: Vec<String>,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: String,
    pub server_name: String,
    pub protocol_version: String,
}

/// Goodbye message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodbyePayload {
    pub reason: String,
    pub code: u32,
}

/// Heartbeat payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub sequence_number: u64,
}

/// Error payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub error_code: u32,
    pub error_message: String,
    pub details: Option<String>,
}

/// Authentication request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequestPayload {
    pub auth_method: String,
    pub credentials: serde_json::Value, // Flexible credential storage
}

/// Authentication response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponsePayload {
    pub success: bool,
    pub session_token: Option<String>,
    pub error_message: Option<String>,
}

/// Video start payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStartPayload {
    pub resolution: VideoResolution,
    pub fps: u32,
    pub quality: String,
    pub codec: String,
}

/// Video frame payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFramePayload {
    pub frame_number: u64,
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub data: Vec<u8>, // Compressed frame data
}

/// Input event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEventPayload {
    pub event_type: String,
    pub data: serde_json::Value, // Flexible input data
}

/// Clipboard payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub data_type: String, // "text", "image", etc.
    pub data: Vec<u8>,
}

/// Service announcement payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAnnouncementPayload {
    pub service_id: String,
    pub service_name: String,
    pub service_type: ServiceType,
    pub address: NetworkAddress,
    pub capabilities: Vec<String>,
}

/// Service query payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceQueryPayload {
    pub query_type: String,
    pub filters: Option<serde_json::Value>,
}

/// Service response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponsePayload {
    pub services: Vec<ServiceAnnouncementPayload>,
}

/// Metrics payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metrics: serde_json::Value,
}

impl ProtocolMessage {
    /// Create a new protocol message
    pub fn new(message_type: MessageType, payload: MessagePayload) -> Self {
        let header = MessageHeader {
            message_type,
            message_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            session_id: None,
            compression: false,
            payload_size: 0, // Will be calculated when serialized
        };

        ProtocolMessage { header, payload }
    }

    /// Create a new message with session ID
    pub fn with_session(mut self, session_id: String) -> Self {
        self.header.session_id = Some(session_id);
        self
    }

    /// Set compression flag
    pub fn with_compression(mut self, compression: bool) -> Self {
        self.header.compression = compression;
        self
    }

    /// Get message type
    pub fn message_type(&self) -> &MessageType {
        &self.header.message_type
    }

    /// Get session ID
    pub fn session_id(&self) -> Option<&String> {
        self.header.session_id.as_ref()
    }

    /// Check if message is compressed
    pub fn is_compressed(&self) -> bool {
        self.header.compression
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        MessageHeader {
            message_type: MessageType::Hello,
            message_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            session_id: None,
            compression: false,
            payload_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_message_creation() {
        let payload = MessagePayload::Hello(HelloPayload {
            protocol_version: "1.0.0".to_string(),
            client_info: ClientInfo {
                client_id: "test-client".to_string(),
                client_name: "Test Client".to_string(),
                platform: "linux".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: vec!["video".to_string(), "input".to_string()],
        });

        let message = ProtocolMessage::new(MessageType::Hello, payload);

        assert_eq!(message.message_type(), &MessageType::Hello);
        assert!(message.session_id().is_none());
        assert!(!message.is_compressed());
    }

    #[test]
    fn test_message_with_session() {
        let payload = MessagePayload::Heartbeat(HeartbeatPayload {
            sequence_number: 42,
        });

        let message = ProtocolMessage::new(MessageType::Heartbeat, payload)
            .with_session("session-123".to_string());

        assert_eq!(message.session_id(), Some(&"session-123".to_string()));
    }
}
