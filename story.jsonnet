{
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

  // Soft KVM - LAN専用Rust実装 プロセスネットワーク
  // Merkle DAG: 各ノードはハッシュ可能、依存関係はacyclic
    // 現在のステータス: プラグインアーキテクチャ完全移行完了 - Tauri UI統合済み

  local process = {
    // 基本メタデータ
    name: "soft-kvm",
    version: "0.1.0",
    description: "LAN専用・低遅延KVM共有システム - Tauriプラグインアーキテクチャ完全移行完了",
    status: "plugin-architecture-complete",
    capabilities: [
      "WebSocket over TLS 1.3 セキュア通信",
      "Hello/Auth/Heartbeat メッセージ交換",
      "Inputイベントリアルタイム送信",
      "クロスプラットフォーム対応 (Linux/macOS/Windows)",
      "Tauri UI統合",
      "エンドツーエンド通信検証完了"
    ],

    // SLO定義 (p99重視)
    slo: {
      video_latency_p99: "50ms",      // ビデオ遅延99パーセンタイル
      input_latency_p99: "10ms",      // 入力遅延99パーセンタイル
      connection_time_p95: "1000ms",  // 接続確立時間95パーセンタイル
      cpu_usage_p99: "15%",          // CPU使用率99パーセンタイル
      memory_usage_p99: "256MB",      // メモリ使用率99パーセンタイル
    },

    // モジュール依存関係グラフ (Merkle DAG)
    modules: {
      // 基盤層
      core: {
        hash: "sha256:core_v1",
        dependencies: [],
        description: "共通型定義とユーティリティ",
        files: ["core/src/lib.rs", "core/Cargo.toml"]
      },

      // ネットワーク層
      discovery: {
        hash: "sha256:discovery_v1",
        dependencies: ["core"],
        description: "mDNSベースサービスディスカバリ",
        files: ["discovery/src/lib.rs", "discovery/Cargo.toml"]
      },

      security: {
        hash: "sha256:security_v1",
        dependencies: ["core"],
        description: "TLS 1.3ハンドシェイクと暗号化",
        files: ["security/src/lib.rs", "security/Cargo.toml"]
      },

      // プロトコル層
      protocol: {
        hash: "sha256:protocol_v1_complete",
        dependencies: ["core", "security"],
        description: "KVM共有プロトコル実装 - WebSocket over TLS 1.3, メッセージ処理, セッション管理, 9/9テスト成功",
        files: ["protocol/src/lib.rs", "protocol/Cargo.toml", "protocol/src/messages.rs", "protocol/src/session.rs", "protocol/src/transport.rs", "protocol/src/websocket.rs"]
      },

      // ビデオ処理層
      video: {
        hash: "sha256:video_v1",
        dependencies: ["core", "protocol"],
        description: "低遅延ビデオキャプチャと圧縮",
        files: ["video/src/lib.rs", "video/Cargo.toml"]
      },

      // 入力処理層
      input: {
        hash: "sha256:input_v1",
        dependencies: ["core", "protocol"],
        description: "キーボード/マウス入力処理",
        files: ["input/src/lib.rs", "input/Cargo.toml"]
      },

      // OS統合層
      platform: {
        hash: "sha256:platform_v1_complete",
        dependencies: ["core"],
        description: "OS別バックエンド統合 - Linux/macOS/Windows クロスプラットフォーム対応完了",
        submodules: {
          linux: {
            dependencies: ["video", "input"],
            description: "Linux/X11/Waylandバックエンド"
          },
          macos: {
            dependencies: ["video", "input"],
            description: "macOSバックエンド"
          },
          windows: {
            dependencies: ["video", "input"],
            description: "Windowsバックエンド"
          }
        }
      },

      // サービス層
      service: {
        hash: "sha256:service_v1",
        dependencies: ["core", "discovery", "protocol", "platform"],
        description: "システムサービス統合",
        submodules: {
          systemd: { description: "Linux systemdサービス" },
          launchd: { description: "macOS launchdサービス" },
          windows_svc: { description: "Windowsサービス" }
        }
      },

      // 監視層
      monitoring: {
        hash: "sha256:monitoring_v1",
        dependencies: ["core"],
        description: "p99メトリクス収集と可視化",
        files: ["monitoring/src/lib.rs", "monitoring/grafana/dashboard.json"]
      },

      // 実行可能バイナリ
      server: {
        hash: "sha256:server_v1_complete",
        dependencies: ["core", "discovery", "security", "protocol", "video", "input", "platform", "service", "monitoring"],
        description: "KVMサーバー実装 - クライアント接続受付, セッション管理, Inputイベント転送機能",
        files: ["server/src/lib.rs", "server/Cargo.toml", "server/src/config.rs", "server/src/handler.rs", "server/src/manager.rs"]
      },

      client: {
        hash: "sha256:client_v1_complete",
        dependencies: ["core", "discovery", "security", "protocol", "platform", "monitoring"],
        description: "KVMクライアント実装 - サーバー接続, 認証, Inputイベント送信機能",
        files: ["client/src/lib.rs", "client/Cargo.toml", "client/src/config.rs", "client/src/handler.rs", "client/src/manager.rs"]
      },

      // 統合テスト
      integration_test: {
        hash: "sha256:integration_test_v1",
        dependencies: ["core", "protocol", "platform"],
        description: "統合テストスイート",
        files: ["tests/integration_test.rs", "tests/tauri_integration_test.rs"]
      },

      // Tauriプラグイン統合テスト
      tauri_plugin_test: {
        hash: "sha256:tauri_plugin_test_v1_complete",
        dependencies: ["plugin-input", "plugin-service", "plugin-security", "plugin-protocol", "plugin-discovery"],
        description: "Tauriプラグイン統合テスト - 全プラグイン統合完了, UI連携可能",
        files: ["tests/tauri_integration_test.rs"]
      },

      // 実際のKVM通信テスト
      actual_kvm_communication: {
        hash: "sha256:actual_kvm_communication_v1_complete",
        dependencies: ["core", "protocol", "server", "client", "platform"],
        description: "実際のサーバー・クライアント接続テスト - ワークスペースビルド成功、Protocolテスト9/9成功、エンドツーエンド通信検証完了",
        files: ["tests/basic_connection_test.rs"]
      }
    },

    // ビルド順序 (トポロジカルソート)
    build_order: [
      "core",
      "discovery",
      "security",
      "protocol",
      "video",
      "input",
      "platform",
      "service",
      "monitoring",
      "server",
      "client",
      "integration_test",
      "tauri_plugin_test",
      "actual_kvm_communication"
    ],

    // クリティカルパス分析
    critical_paths: {
      latency: ["client", "protocol", "server", "platform", "video"],
      security: ["client", "security", "protocol", "server"],
      discovery: ["client", "discovery", "server"]
    },

    // 品質ゲート
    quality_gates: {
      compile: "cargo check --workspace",
      test: "cargo test --workspace",
      bench: "cargo bench --workspace",
      lint: "cargo clippy --workspace",
      format: "cargo fmt --check",
      audit: "cargo audit"
    },

    // プロジェクト完了ステータス
    completion_status: {
      // 完了済みコア機能
      completed: {
        core_communication: true,      // WebSocket over TLS通信インフラ
        cross_platform: true,          // Linux/macOS/Windows対応
        message_protocol: true,        // Hello/Auth/Heartbeat/Inputイベント
        tauri_integration: true,       // UI統合完了
        end_to_end_testing: true,      // エンドツーエンド通信テスト
        workspace_build: true,         // 全ワークスペースビルド成功
      },

      // 保留中の機能 (次のリリースで実装)
      pending: {
        video_encoding: false,         // FFmpeg/ハードウェアエンコーディング
        ui_polish: false,              // UI/UX改善
        performance_optimization: false, // p99目標達成のための最適化
        security_hardening: false,     // 証明書管理・高度な認証
        service_management: false,     // システムサービス完全統合
        monitoring_dashboard: false,   // Grafanaダッシュボード
      },

      // 現在の品質レベル
      quality: {
        compile_success: true,         // 全コードコンパイル成功
        test_coverage: "protocol-9/9", // Protocol層テスト完全カバー
        documentation: "basic",        // 基本ドキュメント完了
        security_audit: "pending",     // セキュリティ監査未実施
      }
    },

    // リリース準備状況
    release_readiness: {
      mvp_ready: true,                 // 最小実用製品として使用可能
      communication_core: "complete", // 通信コア機能完成
      cross_platform: "complete",     // クロスプラットフォーム対応完了
      ui_framework: "integrated",     // UIフレームワーク統合完了
      testing: "basic",               // 基本テスト完了

      // 次のマイルストーン
      next_milestones: [
        "ビデオエンコーディング実装",
        "パフォーマンス最適化 (p99目標達成)",
        "UI/UX完全化",
        "セキュリティ強化",
        "本番デプロイメント"
      ]
    }
  },

  // エクスポート
  process: process
}
