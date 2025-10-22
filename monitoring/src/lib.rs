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

//! # Soft KVM Monitoring
//!
//! p99メトリクス収集と可視化実装
//!
//! ## Merkle DAG Node
//! hash: sha256:monitoring_v1
//! dependencies: [core]

use soft_kvm_core::Metrics;
use metrics::{counter, histogram, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;

/// メトリクス収集器
#[derive(Clone)]
pub struct MetricsCollector {
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        // Prometheusエクスポーターを初期化
        let builder = PrometheusBuilder::new();
        builder.install().expect("Failed to install Prometheus recorder");

        Self {
            start_time: Instant::now(),
        }
    }

    /// ビデオ遅延を記録
    pub fn record_video_latency(&self, latency_ms: f64) {
        histogram!("soft_kvm_video_latency_ms").record(latency_ms);
    }

    /// 入力遅延を記録
    pub fn record_input_latency(&self, latency_ms: f64) {
        histogram!("soft_kvm_input_latency_ms").record(latency_ms);
    }

    /// CPU使用率を記録
    pub fn record_cpu_usage(&self, usage_percent: f64) {
        histogram!("soft_kvm_cpu_usage_percent").record(usage_percent);
    }

    /// メモリ使用量を記録
    pub fn record_memory_usage(&self, usage_mb: f64) {
        histogram!("soft_kvm_memory_usage_mb").record(usage_mb);
    }

    /// ネットワーク使用量を記録
    pub fn record_network_usage(&self, bytes_per_sec: u64) {
        gauge!("soft_kvm_network_bytes_per_sec").set(bytes_per_sec as f64);
    }

    /// 接続時間を記録
    pub fn record_connection_time(&self, time_ms: f64) {
        histogram!("soft_kvm_connection_time_ms").record(time_ms);
    }

    /// アクティブ接続数を記録
    pub fn record_active_connections(&self, count: usize) {
        gauge!("soft_kvm_active_connections").set(count as f64);
    }

    /// エラーを記録
    pub fn record_error(&self, error_type: &str) {
        let error_type: &'static str = Box::leak(error_type.to_string().into_boxed_str());
        counter!("soft_kvm_errors_total", "type" => error_type).increment(1);
    }

    /// ハンドシェイク時間を記録
    pub fn record_handshake_time(&self, time_ms: f64) {
        histogram!("soft_kvm_handshake_time_ms").record(time_ms);
    }

    /// アップタイムを記録
    pub fn record_uptime(&self) {
        let uptime = self.start_time.elapsed().as_secs_f64();
        gauge!("soft_kvm_uptime_seconds").set(uptime);
    }

    /// 情報を記録
    pub fn record_info(&self, version: &str, build: &str) {
        gauge!("soft_kvm_info", "version" => version.to_string(), "build" => build.to_string()).set(1.0);
    }

    /// 詳細メトリクスを記録
    pub fn record_detailed_metrics(&self, metrics: &Metrics) {
        self.record_video_latency(metrics.video_latency_ms);
        self.record_input_latency(metrics.input_latency_ms);
        self.record_cpu_usage(metrics.cpu_usage_percent);
        self.record_memory_usage(metrics.memory_usage_mb);
        self.record_network_usage(metrics.network_bytes_per_sec);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// パフォーマンスモニター
pub struct PerformanceMonitor {
    collector: MetricsCollector,
    last_measurement: Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            collector: MetricsCollector::new(),
            last_measurement: Instant::now(),
        }
    }

    /// パフォーマンス測定を開始
    pub fn start_measurement(&mut self) -> Measurement {
        Measurement::new(self.collector.clone())
    }

    /// 定期的なメトリクス更新
    pub async fn start_periodic_updates(&self) {
        let collector = self.collector.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;
                collector.record_uptime();
            }
        });
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 測定ハンドル
pub struct Measurement {
    collector: MetricsCollector,
    start_time: Instant,
    operation: String,
}

impl Measurement {
    pub fn new(collector: MetricsCollector) -> Self {
        Self {
            collector,
            start_time: Instant::now(),
            operation: "unknown".to_string(),
        }
    }

    pub fn with_operation(mut self, operation: &str) -> Self {
        self.operation = operation.to_string();
        self
    }

    pub fn record_video_operation(self) -> VideoMeasurement {
        VideoMeasurement {
            measurement: self,
        }
    }

    pub fn record_input_operation(self) -> InputMeasurement {
        InputMeasurement {
            measurement: self,
        }
    }

    pub fn finish(self) {
        let duration = self.start_time.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!("Operation '{}' completed in {:.2}ms", self.operation, duration);
    }
}

/// ビデオ測定
pub struct VideoMeasurement {
    measurement: Measurement,
}

impl VideoMeasurement {
    pub fn finish(self) {
        let duration = self.measurement.start_time.elapsed().as_secs_f64() * 1000.0;
        self.measurement.collector.record_video_latency(duration);
        tracing::debug!("Video operation completed in {:.2}ms", duration);
    }
}

/// 入力測定
pub struct InputMeasurement {
    measurement: Measurement,
}

impl InputMeasurement {
    pub fn finish(self) {
        let duration = self.measurement.start_time.elapsed().as_secs_f64() * 1000.0;
        self.measurement.collector.record_input_latency(duration);
        tracing::debug!("Input operation completed in {:.2}ms", duration);
    }
}