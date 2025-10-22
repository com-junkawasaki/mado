//! ユーティリティ関数

use crate::KvmResult;
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

/// 時間計測ユーティリティ
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1000.0
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

/// パフォーマンス統計
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub count: u64,
    pub total_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub p50_time_ms: f64,
    pub p95_time_ms: f64,
    pub p99_time_ms: f64,
}

impl PerformanceStats {
    pub fn new() -> Self {
        Self {
            count: 0,
            total_time_ms: 0.0,
            min_time_ms: f64::INFINITY,
            max_time_ms: 0.0,
            p50_time_ms: 0.0,
            p95_time_ms: 0.0,
            p99_time_ms: 0.0,
        }
    }

    pub fn record(&mut self, duration_ms: f64) {
        self.count += 1;
        self.total_time_ms += duration_ms;
        self.min_time_ms = self.min_time_ms.min(duration_ms);
        self.max_time_ms = self.max_time_ms.max(duration_ms);

        // 簡易パーセンタイル計算（実際の実装ではより正確なアルゴリズムが必要）
        self.p50_time_ms = self.total_time_ms / self.count as f64;
        self.p95_time_ms = self.max_time_ms * 0.95;
        self.p99_time_ms = self.max_time_ms * 0.99;
    }

    pub fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_time_ms / self.count as f64
        }
    }
}

/// ネットワークユーティリティ
pub mod network {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    /// LANアドレスかチェック
    pub fn is_lan_address(addr: &SocketAddr) -> bool {
        match addr.ip() {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                (octets[0] == 10)
                    || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                    || (octets[0] == 192 && octets[1] == 168)
                    || ipv4 == Ipv4Addr::LOCALHOST
            }
            IpAddr::V6(_) => false, // IPv6は未サポート
        }
    }

    /// 利用可能なポートを検索
    pub fn find_available_port(start_port: u16) -> KvmResult<u16> {
        use std::net::TcpListener;

        for port in start_port..65535 {
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(port);
            }
        }
        Err(crate::KvmError::Network("No available ports found".to_string()))
    }
}

/// 文字列ユーティリティ
pub mod string {
    use super::*;

    /// サービス名の正規化
    pub fn normalize_service_name(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .to_lowercase()
    }

    /// バージョン文字列の検証
    pub fn validate_version(version: &str) -> KvmResult<()> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(crate::KvmError::Config("Invalid version format".to_string()));
        }

        for part in parts {
            if part.parse::<u32>().is_err() {
                return Err(crate::KvmError::Config("Invalid version number".to_string()));
            }
        }

        Ok(())
    }
}

/// バッファユーティリティ
pub mod buffer {
    use super::*;

    /// リングバッファ実装
    pub struct RingBuffer<T> {
        buffer: Vec<T>,
        capacity: usize,
        head: usize,
        tail: usize,
        size: usize,
    }

    impl<T> RingBuffer<T> {
        pub fn new(capacity: usize) -> Self {
            Self {
                buffer: Vec::with_capacity(capacity),
                capacity,
                head: 0,
                tail: 0,
                size: 0,
            }
        }

        pub fn push(&mut self, item: T) {
            if self.size < self.capacity {
                self.buffer.push(item);
                self.size += 1;
            } else {
                self.buffer[self.tail] = item;
                self.tail = (self.tail + 1) % self.capacity;
            }
            self.head = (self.head + 1) % self.capacity;
        }

        pub fn iter(&self) -> impl Iterator<Item = &T> {
            (0..self.size).map(move |i| &self.buffer[(self.tail + i) % self.capacity])
        }
    }
}
