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

//! Transport layer abstraction

use crate::{messages::ProtocolMessage, ProtocolResult};
use async_trait::async_trait;
use tracing::{debug, info, warn, error};
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// Transport connection trait
#[async_trait]
pub trait TransportConnection: Send + Sync {
    /// Send a message
    async fn send(&mut self, message: ProtocolMessage) -> ProtocolResult<()>;

    /// Receive a message
    async fn receive(&mut self) -> ProtocolResult<Option<ProtocolMessage>>;

    /// Close the connection
    async fn close(&mut self) -> ProtocolResult<()>;

    /// Get remote address
    fn remote_addr(&self) -> Option<SocketAddr>;

    /// Check if connection is alive
    fn is_alive(&self) -> bool;
}

/// Transport listener trait
#[async_trait]
pub trait TransportListener: Send + Sync {
    /// Accept a new connection
    async fn accept(&mut self) -> ProtocolResult<Box<dyn TransportConnection>>;

    /// Get local address
    fn local_addr(&self) -> Option<SocketAddr>;

    /// Close the listener
    async fn close(&mut self) -> ProtocolResult<()>;
}

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub max_connections: usize,
    pub read_timeout: u64,  // seconds
    pub write_timeout: u64, // seconds
    pub buffer_size: usize,
    pub compression: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            max_connections: 100,
            read_timeout: 30,
            write_timeout: 30,
            buffer_size: 64 * 1024, // 64KB
            compression: true,
        }
    }
}

/// Transport factory trait
#[async_trait]
pub trait TransportFactory: Send + Sync {
    /// Create a new listener
    async fn create_listener(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportListener>>;

    /// Create a new connection
    async fn create_connection(&self, addr: SocketAddr, config: TransportConfig) -> ProtocolResult<Box<dyn TransportConnection>>;
}

/// Message handler trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle_message(&mut self, message: ProtocolMessage, sender: mpsc::UnboundedSender<ProtocolMessage>) -> ProtocolResult<()>;

    /// Handle connection opened
    async fn on_connection_opened(&mut self, remote_addr: SocketAddr) -> ProtocolResult<()> {
        Ok(())
    }

    /// Handle connection closed
    async fn on_connection_closed(&mut self, remote_addr: SocketAddr) -> ProtocolResult<()> {
        Ok(())
    }

    /// Handle connection error
    async fn on_connection_error(&mut self, error: crate::ProtocolError, remote_addr: Option<SocketAddr>) -> ProtocolResult<()> {
        Ok(())
    }
}

/// Transport manager for handling multiple connections
pub struct TransportManager<F, H> {
    factory: F,
    handler: H,
    config: TransportConfig,
    connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<SocketAddr, Box<dyn TransportConnection>>>>,
}

impl<F, H> TransportManager<F, H>
where
    F: TransportFactory + Clone + 'static,
    H: MessageHandler + Clone + 'static,
{
    /// Create a new transport manager
    pub fn new(factory: F, handler: H, config: TransportConfig) -> Self {
        TransportManager {
            factory,
            handler,
            config,
            connections: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Start listening for connections
    pub async fn listen(&self, addr: SocketAddr) -> ProtocolResult<()> {
        let mut listener = self.factory.create_listener(addr, self.config.clone()).await?;

        info!("Transport manager listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok(mut connection) => {
                    let remote_addr = connection.remote_addr()
                        .ok_or_else(|| crate::ProtocolError::Transport("No remote address".to_string()))?;

                    // Handle connection opened
                    self.handler.clone().on_connection_opened(remote_addr).await?;

                    // Store connection
                    {
                        let mut connections = self.connections.write().await;
                        connections.insert(remote_addr, connection);
                    }

                    // Spawn connection handler
                    let connections = self.connections.clone();
                    let mut handler = self.handler.clone();

                    tokio::spawn(async move {
                        let (tx, mut rx) = mpsc::unbounded_channel();

                        // Message processing loop
                        loop {
                            tokio::select! {
                                // Receive message from connection
                                result = async {
                                    let connections = connections.read().await;
                                    if let Some(connection) = connections.get(&remote_addr) {
                                        // Note: This is a simplified implementation
                                        // In practice, we'd need to make the connection clonable or use different approach
                                        Ok(None)
                                    } else {
                                        Err(crate::ProtocolError::Transport("Connection not found".to_string()))
                                    }
                                } => {
                                    match result {
                                        Ok(Some(message)) => {
                                            if let Err(e) = handler.handle_message(message, tx.clone()).await {
                                                error!("Failed to handle message: {}", e);
                                            }
                                        }
                                        Ok(None) => continue,
                                        Err(e) => {
                                            error!("Connection error: {}", e);
                                            break;
                                        }
                                    }
                                }

                                // Send message to connection
                                Some(message) = rx.recv() => {
                                    let mut connections = connections.write().await;
                                    if let Some(connection) = connections.get_mut(&remote_addr) {
                                        if let Err(e) = connection.send(message).await {
                                            error!("Failed to send message: {}", e);
                                        }
                                    }
                                }
                            }
                        }

                        // Handle connection closed
                        {
                            let mut connections = connections.write().await;
                            connections.remove(&remote_addr);
                        }

                        if let Err(e) = handler.on_connection_closed(remote_addr).await {
                            error!("Failed to handle connection closed: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Connect to a remote address
    pub async fn connect(&self, addr: SocketAddr) -> ProtocolResult<()> {
        let mut connection = self.factory.create_connection(addr, self.config.clone()).await?;

        // Handle connection opened
        self.handler.clone().on_connection_opened(addr).await?;

        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(addr, connection);
        }

        info!("Connected to {}", addr);
        Ok(())
    }

    /// Send message to specific address
    pub async fn send_to(&self, addr: SocketAddr, message: ProtocolMessage) -> ProtocolResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(&addr) {
            connection.send(message).await
        } else {
            Err(crate::ProtocolError::Transport(format!("No connection to {}", addr)))
        }
    }

    /// Broadcast message to all connections
    pub async fn broadcast(&self, message: ProtocolMessage) -> ProtocolResult<()> {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            // Note: This requires connection to be clonable or different approach
            // For now, this is a placeholder
            warn!("Broadcast not fully implemented");
        }
        Ok(())
    }

    /// Close all connections
    pub async fn shutdown(&self) -> ProtocolResult<()> {
        let mut connections = self.connections.write().await;
        for (addr, mut connection) in connections.drain() {
            if let Err(e) = connection.close().await {
                error!("Failed to close connection to {}: {}", addr, e);
            }
        }
        Ok(())
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}
