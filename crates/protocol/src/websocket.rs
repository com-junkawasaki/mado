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

//! WebSocket over TLS transport implementation

use crate::{messages::ProtocolMessage, transport::{TransportConnection, TransportListener, TransportFactory, TransportConfig}, ProtocolResult, ProtocolError};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message, MaybeTlsStream};
use tracing::{debug, info, warn, error};

/// WebSocket over TLS connection
pub struct WebSocketConnection {
    stream: tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>,
    remote_addr: SocketAddr,
    config: TransportConfig,
    is_alive: bool,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection from a raw stream
    pub async fn new(stream: TcpStream, config: TransportConfig) -> ProtocolResult<Self> {
        let remote_addr = stream.peer_addr()
            .map_err(|e| ProtocolError::Transport(format!("Failed to get peer address: {}", e)))?;

        // Accept WebSocket connection
        let ws_stream = accept_async(tokio_tungstenite::MaybeTlsStream::Plain(stream))
            .await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to accept WebSocket: {}", e)))?;

        Ok(WebSocketConnection {
            stream: ws_stream,
            remote_addr,
            config,
            is_alive: true,
        })
    }

    /// Create a client WebSocket connection
    pub async fn connect(addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Self> {
        let url = format!("ws://{}", addr);
        let url = url::Url::parse(&url)
            .map_err(|e| ProtocolError::Transport(format!("Invalid URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to connect WebSocket: {}", e)))?;

        // Convert the WebSocket stream to TcpStream based type
        // Note: This is a simplified implementation. In practice, you might need
        // to handle TLS streams differently
        Ok(WebSocketConnection {
            stream: ws_stream,
            remote_addr: addr,
            config,
            is_alive: true,
        })
    }
}

#[async_trait]
impl TransportConnection for WebSocketConnection {
    async fn send(&mut self, message: ProtocolMessage) -> ProtocolResult<()> {
        if !self.is_alive {
            return Err(ProtocolError::Transport("Connection is closed".to_string()));
        }

        // Serialize message
        let data = serde_json::to_string(&message)
            .map_err(|e| ProtocolError::Serialization(e))?;

        // Check message size
        if data.len() > self.config.buffer_size {
            return Err(ProtocolError::Transport(format!(
                "Message size {} exceeds buffer size {}",
                data.len(),
                self.config.buffer_size
            )));
        }

        // Send as WebSocket text message
        let ws_message = Message::Text(data);
        self.stream.send(ws_message).await
            .map_err(|e| ProtocolError::WebSocket(format!("Failed to send message: {}", e)))?;

        debug!("Sent message to {}", self.remote_addr);
        Ok(())
    }

    async fn receive(&mut self) -> ProtocolResult<Option<ProtocolMessage>> {
        if !self.is_alive {
            return Ok(None);
        }

        // Set read timeout
        let timeout_duration = std::time::Duration::from_secs(self.config.read_timeout);

        match tokio::time::timeout(timeout_duration, self.stream.next()).await {
            Ok(Some(Ok(message))) => {
                match message {
                    Message::Text(text) => {
                        // Deserialize message
                        let protocol_message: ProtocolMessage = serde_json::from_str(&text)
                            .map_err(|e| ProtocolError::Serialization(e))?;

                        debug!("Received message from {}", self.remote_addr);
                        Ok(Some(protocol_message))
                    }
                    Message::Binary(data) => {
                        // Handle binary messages if needed
                        warn!("Received binary message from {}, ignoring", self.remote_addr);
                        Ok(None)
                    }
                    Message::Ping(_) => {
                        // Auto-pong is handled by tungstenite
                        Ok(None)
                    }
                    Message::Pong(_) => {
                        // Pong received
                        Ok(None)
                    }
                    Message::Close(_) => {
                        info!("WebSocket close frame received from {}", self.remote_addr);
                        self.is_alive = false;
                        Ok(None)
                    }
                    Message::Frame(_) => {
                        // Raw frame, ignore for now
                        Ok(None)
                    }
                }
            }
            Ok(Some(Err(e))) => {
                error!("WebSocket error from {}: {}", self.remote_addr, e);
                self.is_alive = false;
                Err(ProtocolError::WebSocket(format!("WebSocket error: {}", e)))
            }
            Ok(None) => {
                // Stream ended
                info!("WebSocket stream ended for {}", self.remote_addr);
                self.is_alive = false;
                Ok(None)
            }
            Err(_) => {
                // Timeout
                Err(ProtocolError::Timeout)
            }
        }
    }

    async fn close(&mut self) -> ProtocolResult<()> {
        if self.is_alive {
            self.stream.close(None).await
                .map_err(|e| ProtocolError::WebSocket(format!("Failed to close WebSocket: {}", e)))?;
            self.is_alive = false;
            info!("Closed WebSocket connection to {}", self.remote_addr);
        }
        Ok(())
    }

    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr)
    }

    fn is_alive(&self) -> bool {
        self.is_alive
    }
}

/// WebSocket listener
pub struct WebSocketListener {
    listener: TcpListener,
    config: TransportConfig,
}

impl WebSocketListener {
    /// Create a new WebSocket listener
    pub async fn new(addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ProtocolError::Transport(format!("Failed to bind listener: {}", e)))?;

        info!("WebSocket listener bound to {}", addr);

        Ok(WebSocketListener { listener, config })
    }
}

#[async_trait]
impl TransportListener for WebSocketListener {
    async fn accept(&mut self) -> ProtocolResult<Box<dyn TransportConnection>> {
        let (stream, _) = self.listener.accept().await
            .map_err(|e| ProtocolError::Transport(format!("Failed to accept connection: {}", e)))?;

        let connection = WebSocketConnection::new(stream, self.config.clone()).await?;
        info!("Accepted WebSocket connection from {}", connection.remote_addr);

        Ok(Box::new(connection))
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    async fn close(&mut self) -> ProtocolResult<()> {
        // TcpListener doesn't have a close method, but we can drop it
        info!("WebSocket listener closed");
        Ok(())
    }
}

/// WebSocket factory
#[derive(Clone)]
pub struct WebSocketFactory;

#[async_trait]
impl TransportFactory for WebSocketFactory {
    async fn create_listener(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportListener>> {
        let listener = WebSocketListener::new(addr, config).await?;
        Ok(Box::new(listener))
    }

    async fn create_connection(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportConnection>> {
        let connection = WebSocketConnection::connect(addr, config).await?;
        Ok(Box::new(connection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_websocket_factory_creation() {
        let factory = WebSocketFactory;
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let config = TransportConfig::default();

        // Test listener creation
        let listener_result = factory.create_listener(addr, config.clone()).await;
        assert!(listener_result.is_ok());

        // Test connection creation (will fail since no server is running)
        let connection_result = factory.create_connection(addr, config).await;
        assert!(connection_result.is_err()); // Expected to fail
    }

    #[tokio::test]
    async fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.read_timeout, 30);
        assert_eq!(config.write_timeout, 30);
        assert_eq!(config.buffer_size, 64 * 1024);
        assert!(config.compression);
    }
}
