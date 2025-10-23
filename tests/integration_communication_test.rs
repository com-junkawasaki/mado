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

//! 統合通信テスト - 実際のサーバー・クライアント接続テスト

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

use soft_kvm_core::*;
use soft_kvm_server::{KvmServer, ServerConfig};
use soft_kvm_client::{KvmClient, ClientConfig};

#[tokio::test]
async fn test_server_client_connection() {
    // サーバー設定
    let server_config = ServerConfig {
        server_name: "Test Server".to_string(),
        bind_address: NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081, // テスト用ポート
        },
        max_clients: 5,
        session_timeout: 300,
        heartbeat_interval: 30,
        enable_discovery: false,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
        max_message_size: 1024 * 1024,
    };

    // クライアント設定
    let client_config = ClientConfig {
        client_name: "Test Client".to_string(),
        server_address: Some(NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8081,
        }),
        auto_connect: false,
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
    };

    // サーバーの起動
    let server = KvmServer::new(server_config.clone()).await.unwrap();
    let server_handle = Arc::new(RwLock::new(server));
    let server_clone = server_handle.clone();

    // サーバーを別タスクで起動
    let server_task = tokio::spawn(async move {
        let mut server = server_clone.write().await;
        server.start().await.unwrap();
        println!("Server started successfully");

        // 少し待ってから停止
        tokio::time::sleep(Duration::from_millis(100)).await;
        server.stop().await.unwrap();
        println!("Server stopped successfully");
    });

    // 少し待ってサーバーが起動するのを待つ
    tokio::time::sleep(Duration::from_millis(50)).await;

    // クライアントの作成と接続
    let client = KvmClient::new(client_config.clone()).await.unwrap();
    let client_handle = Arc::new(RwLock::new(client));
    let client_clone = client_handle.clone();

    // クライアントを別タスクで接続
    let client_task = tokio::spawn(async move {
        let mut client = client_clone.write().await;

        let server_addr: SocketAddr = format!("{}:{}", server_config.bind_address.ip, server_config.bind_address.port)
            .parse().unwrap();

        client.connect(server_addr).await.unwrap();
        println!("Client connected successfully");

        // 少し待ってから切断
        tokio::time::sleep(Duration::from_millis(50)).await;
        client.disconnect().await.unwrap();
        println!("Client disconnected successfully");
    });

    // 両方のタスクが完了するのを待つ（タイムアウト付き）
    let result = timeout(Duration::from_secs(5), async {
        let (server_result, client_result) = tokio::join!(server_task, client_task);
        server_result.unwrap();
        client_result.unwrap();
    }).await;

    match result {
        Ok(_) => println!("Server-Client integration test passed!"),
        Err(_) => panic!("Test timed out - possible connection issues"),
    }
}

#[tokio::test]
async fn test_multiple_clients_connection() {
    // サーバー設定
    let server_config = ServerConfig {
        server_name: "Multi-Client Test Server".to_string(),
        bind_address: NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8082, // 別のポート
        },
        max_clients: 3,
        session_timeout: 300,
        heartbeat_interval: 30,
        enable_discovery: false,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
        max_message_size: 1024 * 1024,
    };

    // サーバーの起動
    let server = KvmServer::new(server_config.clone()).await.unwrap();
    let server_handle = Arc::new(RwLock::new(server));

    let server_task = tokio::spawn(async move {
        let mut server = server_handle.write().await;
        server.start().await.unwrap();
        println!("Multi-client server started");

        // 複数クライアントが接続する時間を待つ
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = server.status().await.unwrap();
        println!("Server status: {} clients connected", status.active_sessions);
        assert!(status.active_sessions >= 2, "Expected at least 2 clients to be connected");

        server.stop().await.unwrap();
        println!("Multi-client server stopped");
    });

    // サーバーが起動するのを待つ
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 複数のクライアントを作成
    let mut client_tasks = Vec::new();

    for i in 0..3 {
        let client_config = ClientConfig {
            client_name: format!("Test Client {}", i),
            server_address: Some(NetworkAddress {
                ip: "127.0.0.1".to_string(),
                port: 8082,
            }),
            auto_connect: false,
            max_message_size: 1024 * 1024,
            heartbeat_interval: 30,
            session_timeout: 300,
            enable_security: false,
            video_config: VideoConfig {
                resolution: VideoResolution::fhd(),
                fps: 30,
                quality: VideoQuality::balanced(),
                compression: true,
            },
            input_config: InputConfig {
                enable_keyboard: true,
                enable_mouse: true,
                keyboard_layout: "us".to_string(),
                mouse_sensitivity: 1.0,
            },
        };

        let client = KvmClient::new(client_config).await.unwrap();
        let client_handle = Arc::new(RwLock::new(client));

        let client_task = tokio::spawn(async move {
            let mut client = client_handle.write().await;

            let server_addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
            client.connect(server_addr).await.unwrap();
            println!("Client {} connected", i);

            // 少し待ってから切断
            tokio::time::sleep(Duration::from_millis(100)).await;
            client.disconnect().await.unwrap();
            println!("Client {} disconnected", i);
        });

        client_tasks.push(client_task);
    }

    // すべてのタスクが完了するのを待つ
    let result = timeout(Duration::from_secs(10), async {
        server_task.await.unwrap();

        for (i, task) in client_tasks.into_iter().enumerate() {
            task.await.unwrap();
            println!("Client task {} completed", i);
        }
    }).await;

    match result {
        Ok(_) => println!("Multi-client integration test passed!"),
        Err(_) => panic!("Multi-client test timed out"),
    }
}

#[tokio::test]
async fn test_input_event_communication() {
    // サーバー設定
    let server_config = ServerConfig {
        server_name: "Input Test Server".to_string(),
        bind_address: NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8083,
        },
        max_clients: 1,
        session_timeout: 300,
        heartbeat_interval: 30,
        enable_discovery: false,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
        max_message_size: 1024 * 1024,
    };

    // クライアント設定
    let client_config = ClientConfig {
        client_name: "Input Test Client".to_string(),
        server_address: Some(NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 8083,
        }),
        auto_connect: false,
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
    };

    // サーバーの起動
    let server = KvmServer::new(server_config.clone()).await.unwrap();
    let server_handle = Arc::new(RwLock::new(server));

    let server_task = tokio::spawn(async move {
        let mut server = server_handle.write().await;
        server.start().await.unwrap();
        println!("Input test server started");

        // クライアントが接続してイベントを送る時間を待つ
        tokio::time::sleep(Duration::from_millis(300)).await;

        server.stop().await.unwrap();
        println!("Input test server stopped");
    });

    // サーバーが起動するのを待つ
    tokio::time::sleep(Duration::from_millis(50)).await;

    // クライアントの作成と接続
    let client = KvmClient::new(client_config).await.unwrap();
    let client_handle = Arc::new(RwLock::new(client));

    let client_task = tokio::spawn(async move {
        let mut client = client_handle.write().await;

        let server_addr: SocketAddr = "127.0.0.1:8083".parse().unwrap();
        client.connect(server_addr).await.unwrap();
        println!("Input test client connected");

        // キーボードイベント送信テスト
        let keyboard_event = KeyboardEvent {
            key_code: 65, // 'A' key
            pressed: true,
            modifiers: vec![],
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        client.send_keyboard_event(keyboard_event).await.unwrap();
        println!("Keyboard event sent");

        // マウスイベント送信テスト
        let mouse_event = MouseEvent {
            x: 100,
            y: 200,
            button: Some(1),
            pressed: Some(true),
            wheel_delta: None,
            modifiers: vec![],
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        client.send_mouse_event(mouse_event).await.unwrap();
        println!("Mouse event sent");

        // 少し待ってから切断
        tokio::time::sleep(Duration::from_millis(100)).await;
        client.disconnect().await.unwrap();
        println!("Input test client disconnected");
    });

    // 両方のタスクが完了するのを待つ
    let result = timeout(Duration::from_secs(5), async {
        let (server_result, client_result) = tokio::join!(server_task, client_task);
        server_result.unwrap();
        client_result.unwrap();
    }).await;

    match result {
        Ok(_) => println!("Input event communication test passed!"),
        Err(_) => panic!("Input event test timed out"),
    }
}

#[tokio::test]
async fn test_connection_error_handling() {
    // 存在しないサーバーに接続しようとするテスト
    let client_config = ClientConfig {
        client_name: "Error Test Client".to_string(),
        server_address: Some(NetworkAddress {
            ip: "127.0.0.1".to_string(),
            port: 9999, // 存在しないポート
        }),
        auto_connect: false,
        max_message_size: 1024 * 1024,
        heartbeat_interval: 30,
        session_timeout: 300,
        enable_security: false,
        video_config: VideoConfig {
            resolution: VideoResolution::fhd(),
            fps: 30,
            quality: VideoQuality::balanced(),
            compression: true,
        },
        input_config: InputConfig {
            enable_keyboard: true,
            enable_mouse: true,
            keyboard_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
        },
    };

    let client = KvmClient::new(client_config).await.unwrap();
    let client_handle = Arc::new(RwLock::new(client));

    let client_task = tokio::spawn(async move {
        let mut client = client_handle.write().await;

        let server_addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();

        // 接続を試みるが失敗するはず
        let result = client.connect(server_addr).await;
        assert!(result.is_err(), "Expected connection to fail for non-existent server");
        println!("Connection error handling test passed - correctly failed to connect to non-existent server");
    });

    // タスクが完了するのを待つ
    let result = timeout(Duration::from_secs(2), client_task).await;

    match result {
        Ok(task_result) => {
            task_result.unwrap();
            println!("Connection error handling test passed!");
        }
        Err(_) => panic!("Connection error handling test timed out"),
    }
}
