{
  // Soft KVM - LAN専用Rust実装 プロセスネットワーク
  // Merkle DAG: 各ノードはハッシュ可能、依存関係はacyclic

  local process = {
    // 基本メタデータ
    name: "soft-kvm",
    version: "0.1.0",
    description: "LAN専用・低遅延KVM共有システム",

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
        hash: "sha256:protocol_v1",
        dependencies: ["core", "security"],
        description: "KVM共有プロトコル実装",
        files: ["protocol/src/lib.rs", "protocol/Cargo.toml"]
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
        hash: "sha256:platform_v1",
        dependencies: ["core"],
        description: "OS別バックエンド統合",
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
        hash: "sha256:server_v1",
        dependencies: ["core", "discovery", "security", "protocol", "video", "input", "platform", "service", "monitoring"],
        description: "KVMサーバー実装",
        files: ["server/src/main.rs", "server/Cargo.toml"]
      },

      client: {
        hash: "sha256:client_v1",
        dependencies: ["core", "discovery", "security", "protocol", "platform", "monitoring"],
        description: "KVMクライアント実装",
        files: ["client/src/main.rs", "client/Cargo.toml"]
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
      "client"
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
    }
  },

  // エクスポート
  process: process
}
