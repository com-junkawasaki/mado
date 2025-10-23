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
use std::sync::Arc;
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

/// Connection handle for managing connections
pub struct ConnectionHandle {
    pub remote_addr: SocketAddr,
    pub sender: mpsc::UnboundedSender<ProtocolMessage>,
    pub receiver: mpsc::UnboundedReceiver<ProtocolMessage>,
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

/// TLS configuration for transport
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub enabled: bool,
    pub certificate_path: Option<String>,
    pub private_key_path: Option<String>,
    pub ca_certificate_path: Option<String>,
    pub accept_invalid_certs: bool,
    pub accept_invalid_hostnames: bool,
}

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub max_connections: usize,
    pub read_timeout: u64,  // seconds
    pub write_timeout: u64, // seconds
    pub buffer_size: usize,
    pub compression: bool,
    pub tls: TlsConfig,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            max_connections: 100,
            read_timeout: 30,
            write_timeout: 30,
            buffer_size: 64 * 1024, // 64KB
            compression: true,
            tls: TlsConfig::default(),
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        TlsConfig {
            enabled: false,
            certificate_path: None,
            private_key_path: None,
            ca_certificate_path: None,
            accept_invalid_certs: false,
            accept_invalid_hostnames: false,
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
pub struct TransportManager<F> {
    factory: F,
    config: TransportConfig,
    connections: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<SocketAddr, mpsc::UnboundedSender<ProtocolMessage>>>>,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
}

impl<F> TransportManager<F>
where
    F: TransportFactory + Clone + Send + Sync + 'static,
{
    /// Create a new transport manager
    pub fn new(factory: F, config: TransportConfig) -> Self {
        let (shutdown_sender, _) = tokio::sync::broadcast::channel(1);
        TransportManager {
            factory,
            config,
            connections: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            shutdown_sender,
        }
    }

    /// Start listening for connections
    pub async fn listen<H>(&self, addr: SocketAddr, mut handler: H) -> ProtocolResult<()>
    where
        H: MessageHandler + Send + Sync + Clone + 'static,
    {
        let mut listener = self.factory.create_listener(addr, self.config.clone()).await?;
        let local_addr = listener.local_addr();

        info!("Transport manager listening on {}", local_addr.unwrap_or(addr));

        let connections = Arc::clone(&self.connections);
        let factory = self.factory.clone();
        let config = self.config.clone();
        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok(mut connection) => {
                                let remote_addr = match connection.remote_addr() {
                                    Some(addr) => addr,
                                    None => {
                                        warn!("Connection without remote address");
                                        continue;
                                    }
                                };

                                // Create channels for this connection
                                let (tx_to_connection, mut rx_to_connection) = mpsc::unbounded_channel::<ProtocolMessage>();
                                let (tx_from_connection, rx_from_connection) = mpsc::unbounded_channel::<ProtocolMessage>();

                                // Handle connection opened
                                if let Err(e) = handler.on_connection_opened(remote_addr).await {
                                    error!("Failed to handle connection opened: {}", e);
                                    continue;
                                }

                                // Store sender for this connection
                                let tx_for_storage = tx_from_connection.clone();
                                {
                                    let mut connections = connections.write().await;
                                    connections.insert(remote_addr, tx_for_storage);
                                }

                                // Spawn connection reader
                                let mut connection_reader = connection;
                                let connections_clone = Arc::clone(&connections);
                                let mut handler_clone = handler.clone();
                                let mut shutdown_rx = shutdown_receiver.resubscribe();

                                tokio::spawn(async move {
                                    loop {
                                        tokio::select! {
                                            // Receive message from connection
                                            receive_result = connection_reader.receive() => {
                                                match receive_result {
                                                    Ok(Some(message)) => {
                                                        if let Err(e) = handler_clone.handle_message(message, tx_from_connection.clone()).await {
                                                            error!("Failed to handle message: {}", e);
                                                            break;
                                                        }
                                                    }
                                                    Ok(None) => {
                                                        // Connection closed
                                                        break;
                                                    }
                                                    Err(e) => {
                                                        error!("Connection receive error: {}", e);
                                                        break;
                                                    }
                                                }
                                            }

                                            // Send message to connection
                                            Some(message) = rx_to_connection.recv() => {
                                                if let Err(e) = connection_reader.send(message).await {
                                                    error!("Failed to send message: {}", e);
                                                    break;
                                                }
                                            }

                                            // Shutdown signal
                                            _ = shutdown_rx.recv() => {
                                                break;
                                            }
                                        }
                                    }

                                    // Clean up connection
                                    {
                                        let mut connections = connections_clone.write().await;
                                        connections.remove(&remote_addr);
                                    }

                                    if let Err(e) = handler_clone.on_connection_closed(remote_addr).await {
                                        error!("Failed to handle connection closed: {}", e);
                                    }

                                    // Close connection
                                    if let Err(e) = connection_reader.close().await {
                                        error!("Failed to close connection: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Failed to accept connection: {}", e);
                                break;
                            }
                        }
                    }

                    _ = shutdown_receiver.recv() => {
                        info!("Transport manager shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Connect to a remote address
    pub async fn connect<H>(&self, addr: SocketAddr, mut handler: H) -> ProtocolResult<mpsc::UnboundedReceiver<ProtocolMessage>>
    where
        H: MessageHandler + Send + Sync + Clone + 'static,
    {
        let mut connection = self.factory.create_connection(addr, self.config.clone()).await?;

        // Handle connection opened
        handler.on_connection_opened(addr).await?;

        // Create channels for this connection
        let (tx_to_connection, mut rx_to_connection) = mpsc::unbounded_channel::<ProtocolMessage>();
        let (tx_from_connection, rx_from_connection) = mpsc::unbounded_channel::<ProtocolMessage>();

        // Store sender for this connection
        let tx_for_storage = tx_from_connection.clone();
        {
            let mut connections = self.connections.write().await;
            connections.insert(addr, tx_for_storage);
        }

        // Spawn connection handler
        let connections_clone = Arc::clone(&self.connections);
        let mut handler_clone = handler.clone();
        let mut shutdown_rx = self.shutdown_sender.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Receive message from connection
                    receive_result = connection.receive() => {
                        match receive_result {
                            Ok(Some(message)) => {
                                if let Err(e) = handler_clone.handle_message(message, tx_from_connection.clone()).await {
                                    error!("Failed to handle message: {}", e);
                                    break;
                                }
                            }
                            Ok(None) => {
                                // Connection closed
                                break;
                            }
                            Err(e) => {
                                error!("Connection receive error: {}", e);
                                break;
                            }
                        }
                    }

                    // Send message to connection
                    Some(message) = rx_to_connection.recv() => {
                        if let Err(e) = connection.send(message).await {
                            error!("Failed to send message: {}", e);
                            break;
                        }
                    }

                    // Shutdown signal
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }

            // Clean up connection
            {
                let mut connections = connections_clone.write().await;
                connections.remove(&addr);
            }

            if let Err(e) = handler_clone.on_connection_closed(addr).await {
                error!("Failed to handle connection closed: {}", e);
            }

            // Close connection
            if let Err(e) = connection.close().await {
                error!("Failed to close connection: {}", e);
            }
        });

        info!("Connected to {}", addr);
        Ok(rx_from_connection)
    }

    /// Send message to specific address
    pub async fn send_to(&self, addr: SocketAddr, message: ProtocolMessage) -> ProtocolResult<()> {
        let connections = self.connections.read().await;
        if let Some(sender) = connections.get(&addr) {
            sender.send(message)
                .map_err(|e| crate::ProtocolError::Transport(format!("Failed to send message: {}", e)))?;
        } else {
            return Err(crate::ProtocolError::Transport(format!("No connection to {}", addr)));
        }
        Ok(())
    }

    /// Broadcast message to all connections
    pub async fn broadcast(&self, message: ProtocolMessage) -> ProtocolResult<()> {
        let connections = self.connections.read().await;
        let mut send_errors = Vec::new();

        for (addr, sender) in connections.iter() {
            if let Err(e) = sender.send(message.clone()) {
                send_errors.push(format!("{}: {}", addr, e));
            }
        }

        if !send_errors.is_empty() {
            return Err(crate::ProtocolError::Transport(format!("Broadcast errors: {:?}", send_errors)));
        }

        Ok(())
    }

    /// Close all connections
    pub async fn shutdown(&self) -> ProtocolResult<()> {
        let _ = self.shutdown_sender.send(());
        Ok(())
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}
