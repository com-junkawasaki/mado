# Soft KVM - LAN専用・低遅延KVM共有システム

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

**平均は嘘をつくので、マウスには p99 を見せましょう。**

Soft KVMは、LAN専用・低遅延のKVM（Keyboard, Video, Mouse）共有システムです。Rustで実装され、mDNSベースのサービスディスカバリとTLS 1.3による暗号化を備えています。

## 🎯 SLO (Service Level Objectives)

| メトリクス | 目標値 | 説明 |
|-----------|--------|------|
| ビデオ遅延 p99 | < 50ms | 99パーセンタイルのビデオ遅延 |
| 入力遅延 p99 | < 10ms | 99パーセンタイルの入力遅延 |
| CPU使用率 p99 | < 15% | 99パーセンタイルのCPU使用率 |
| メモリ使用量 p99 | < 256MB | 99パーセンタイルのメモリ使用量 |
| 接続確立時間 p95 | < 1000ms | 95パーセンタイルの接続確立時間 |

## 🏗️ アーキテクチャ

### Merkle DAG プロセスネットワーク

```
story.jsonnet
├── core (共通型定義・ユーティリティ)
├── discovery (mDNSサービスディスカバリ)
├── security (TLS 1.3 ハンドシェイク)
├── protocol (KVM共有プロトコル)
├── video (低遅延ビデオキャプチャ・圧縮)
├── input (キーボード/マウス入力処理)
├── platform (OS別バックエンド統合)
│   ├── linux (Wayland/X11)
│   ├── macos (Quartz)
│   └── windows (DirectX)
├── service (システムサービス統合)
│   ├── systemd (Linux)
│   ├── launchd (macOS)
│   └── windows_svc (Windows)
└── monitoring (p99メトリクス収集・可視化)
```

### プロトコル仕様

- **ディスカバリ**: mDNS (RFC 6762) + TXTレコード
- **セキュリティ**: TLS 1.3 (RFC 8446)
- **ビデオ**: H.264/AVC + RTP/RTCP
- **入力**: 独自バイナリプロトコル
- **制御**: WebSocket over TLS

## 🚀 クイックスタート

### 必要条件

- Rust 1.70+
- pnpm (パッケージ管理)
- Linux/macOS/Windows
- Grafana + Prometheus (監視用)

### ビルド

```bash
# ワークスペース全体をビルド
cargo build --release --workspace

# 個別クレートをビルド
cargo build -p soft-kvm-server
cargo build -p soft-kvm-client
```

### 実行

```bash
# サーバー起動
./target/release/soft-kvm-server

# クライアント起動
./target/release/soft-kvm-client --server <server-name>
```

## 🔧 セットアップ

### Linux (systemd)

```bash
# サービスファイル作成
sudo cp monitoring/grafana/soft-kvm.service /etc/systemd/system/

# サービス有効化・起動
sudo systemctl daemon-reload
sudo systemctl enable soft-kvm
sudo systemctl start soft-kvm

# ログ確認
journalctl -u soft-kvm -f
```

### macOS (launchd)

```bash
# plistファイル作成
sudo cp service/src/launchd.plist /Library/LaunchDaemons/com.soft-kvm.server.plist

# サービス起動
sudo launchctl load /Library/LaunchDaemons/com.soft-kvm.server.plist
sudo launchctl start com.soft-kvm.server

# ログ確認
tail -f /var/log/soft-kvm.log
```

### Windows (サービス)

```bash
# サービスインストール
soft-kvm-server.exe --install-service

# サービス起動
net start "Soft KVM Service"

# イベントログ確認
eventvwr.msc
```

## 🎨 Wayland 権限設定 (Linux)

```bash
# xdg-desktop-portal を使用した権限要求
gdbus call --session \
  --dest org.freedesktop.portal.Desktop \
  --object-path /org/freedesktop/portal/desktop \
  --method org.freedesktop.portal.ScreenCast.CreateSession \
  '{}'
```

## 📊 監視ダッシュボード

### Grafana インポート

1. Grafana UIにアクセス
2. `Create` → `Import` を選択
3. `monitoring/grafana/dashboard.json` をアップロード

### Prometheus 設定

```yaml
scrape_configs:
  - job_name: 'soft-kvm'
    static_configs:
      - targets: ['localhost:9090']  # メトリクスエンドポイント
```

### 主要メトリクス

```prometheus
# p99 ビデオ遅延
histogram_quantile(0.99, rate(soft_kvm_video_latency_ms_bucket[5m]))

# p99 入力遅延
histogram_quantile(0.99, rate(soft_kvm_input_latency_ms_bucket[5m]))

# p99 CPU使用率
histogram_quantile(0.99, rate(soft_kvm_cpu_usage_percent_bucket[5m]))

# p99 メモリ使用量
histogram_quantile(0.99, rate(soft_kvm_memory_usage_mb_bucket[5m]))
```

## 🧪 ベンチマーク

### p99 計測スクリプト

```bash
# ビデオ遅延ベンチマーク
cargo bench --bench video_latency -- --p99

# 入力遅延ベンチマーク
cargo bench --bench input_latency -- --p99

# 総合パフォーマンスベンチマーク
cargo bench --bench comprehensive -- --p99
```

### ベンチマーク結果例

```
Video Latency p99: 42.3ms (Target: <50ms) ✓
Input Latency p99: 8.7ms (Target: <10ms) ✓
CPU Usage p99: 12.4% (Target: <15%) ✓
Memory Usage p99: 198MB (Target: <256MB) ✓
Connection Time p95: 856ms (Target: <1000ms) ✓
```

## 🔒 セキュリティ

- **LAN専用**: インターネット経由のアクセスを禁止
- **TLS 1.3**: すべての通信を暗号化
- **証明書ピニング**: 事前共有鍵による認証
- **権限分離**: 最小権限の原則

## 🐛 トラブルシューティング

### ビデオが表示されない

```bash
# Wayland デバッグ有効化
export WAYLAND_DEBUG=1
export WLR_DEBUG=all

# X11 フォールバック
export DISPLAY=:0
```

### 権限エラー

```bash
# Linux: ポータル権限確認
busctl --user call org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.portal.ScreenCast CreateSession a{sv} 0
```

### パフォーマンス問題

```bash
# CPUプロファイリング
cargo flamegraph --bin soft-kvm-server

# メモリリークチェック
valgrind --tool=memcheck ./target/release/soft-kvm-server
```

## 🤝 貢献

1. Fork してブランチ作成
2. 変更を実装
3. テスト追加
4. `cargo fmt && cargo clippy` でコード品質チェック
5. Pull Request 作成

## 📄 ライセンス

Apache License 2.0

## ⚡ パフォーマンス最適化のヒント

- **ビデオ圧縮**: H.264/AVC with hardware acceleration
- **ゼロコピー**: DMAバッファ共有
- **低遅延**: RTP/RTCP フロー制御
- **p99重視**: 平均ではなく99パーセンタイル最適化

---

**"平均は嘘をつく。p99を見よ。"** - Soft KVM 開発者
