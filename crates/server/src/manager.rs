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

//! Server management utilities

use crate::{ServerResult, ServerError, KvmServer, ServerConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Server manager for handling multiple servers
pub struct ServerManager {
    servers: Arc<RwLock<std::collections::HashMap<String, KvmServer>>>,
}

impl ServerManager {
    /// Create a new server manager
    pub fn new() -> Self {
        ServerManager {
            servers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create and register a new server
    pub async fn create_server(&self, server_id: String, config: ServerConfig) -> ServerResult<()> {
        let mut servers = self.servers.write().await;
        if servers.contains_key(&server_id) {
            return Err(ServerError::Generic(format!("Server {} already exists", server_id)));
        }

        let server = KvmServer::new(config).await?;
        servers.insert(server_id, server);
        Ok(())
    }

    /// Get a server by ID (returns a copy of the ID if found)
    pub async fn get_server(&self, server_id: &str) -> Option<String> {
        let servers = self.servers.read().await;
        servers.contains_key(server_id).then(|| server_id.to_string())
    }

    /// Start a server
    pub async fn start_server(&self, server_id: &str) -> ServerResult<()> {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.start().await?;
        } else {
            return Err(ServerError::Generic(format!("Server {} not found", server_id)));
        }
        Ok(())
    }

    /// Stop a server
    pub async fn stop_server(&self, server_id: &str) -> ServerResult<()> {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.stop().await?;
        } else {
            return Err(ServerError::Generic(format!("Server {} not found", server_id)));
        }
        Ok(())
    }

    /// Remove a server
    pub async fn remove_server(&self, server_id: &str) -> ServerResult<()> {
        let mut servers = self.servers.write().await;
        if servers.remove(server_id).is_none() {
            return Err(ServerError::Generic(format!("Server {} not found", server_id)));
        }
        Ok(())
    }

    /// List all servers
    pub async fn list_servers(&self) -> Vec<String> {
        let servers = self.servers.read().await;
        servers.keys().cloned().collect()
    }

    /// Get server status
    pub async fn get_server_status(&self, server_id: &str) -> ServerResult<crate::ServerStatus> {
        let servers = self.servers.read().await;
        if let Some(server) = servers.get(server_id) {
            server.status().await
        } else {
            Err(ServerError::Generic(format!("Server {} not found", server_id)))
        }
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}
