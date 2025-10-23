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

//! Client management utilities

use crate::{ClientResult, ClientError, KvmClient, ClientConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Client manager for handling multiple clients
pub struct ClientManager {
    clients: Arc<RwLock<std::collections::HashMap<String, KvmClient>>>,
}

impl ClientManager {
    /// Create a new client manager
    pub fn new() -> Self {
        ClientManager {
            clients: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create and register a new client
    pub async fn create_client(&self, client_id: String, config: ClientConfig) -> ClientResult<()> {
        let mut clients = self.clients.write().await;
        if clients.contains_key(&client_id) {
            return Err(ClientError::Generic(format!("Client {} already exists", client_id)));
        }

        let client = KvmClient::new(config).await?;
        clients.insert(client_id, client);
        Ok(())
    }

    /// Get a client by ID (returns a copy of the ID if found)
    pub async fn get_client(&self, client_id: &str) -> Option<String> {
        let clients = self.clients.read().await;
        clients.contains_key(client_id).then(|| client_id.to_string())
    }

    /// Connect a client
    pub async fn connect_client(&self, client_id: &str) -> ClientResult<()> {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            client.connect().await?;
        } else {
            return Err(ClientError::Generic(format!("Client {} not found", client_id)));
        }
        Ok(())
    }

    /// Disconnect a client
    pub async fn disconnect_client(&self, client_id: &str) -> ClientResult<()> {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(client_id) {
            client.disconnect().await?;
        } else {
            return Err(ClientError::Generic(format!("Client {} not found", client_id)));
        }
        Ok(())
    }

    /// Remove a client
    pub async fn remove_client(&self, client_id: &str) -> ClientResult<()> {
        let mut clients = self.clients.write().await;
        if clients.remove(client_id).is_none() {
            return Err(ClientError::Generic(format!("Client {} not found", client_id)));
        }
        Ok(())
    }

    /// List all clients
    pub async fn list_clients(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }

    /// Get client status
    pub async fn get_client_status(&self, client_id: &str) -> ClientResult<crate::ClientStatus> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            client.status().await
        } else {
            Err(ClientError::Generic(format!("Client {} not found", client_id)))
        }
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}
