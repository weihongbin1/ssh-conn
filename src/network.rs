//! 网络连接测试模块

use crate::error::{Result, SshConnError};
use crate::models::SshHost;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{timeout, Instant};

/// 网络检测器
pub struct NetworkProbe {
    /// 默认超时时间（秒）
    default_timeout: u64,
}

impl NetworkProbe {
    /// 创建一个新的网络检测器
    pub fn new() -> Self {
        Self {
            default_timeout: 5,
        }
    }

    /// 设置默认超时时间
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.default_timeout = timeout_secs;
        self
    }

    /// 测试单个主机的连接
    pub async fn test_host(&self, host: &mut SshHost) -> Result<()> {
        host.test_connection().await
    }

    /// 批量测试多个主机的连接
    pub async fn test_hosts(&self, hosts: &mut [SshHost]) -> Vec<Result<()>> {
        use futures::future::join_all;

        let tasks = hosts.iter_mut().map(|host| {
            Box::pin(async {
                host.test_connection().await
            })
        });

        join_all(tasks).await
    }

    /// 测试指定主机名和端口的连接
    pub async fn test_connection(&self, hostname: &str, port: u16, timeout_secs: Option<u64>) -> Result<Duration> {
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(self.default_timeout));
        let addr = format!("{}:{}", hostname, port);
        let start_time = Instant::now();

        match timeout(timeout_duration, TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => {
                let duration = start_time.elapsed();
                log::debug!("Connection to {} successful in {:?}", addr, duration);
                Ok(duration)
            }
            Ok(Err(e)) => {
                let error_msg = format!("Connection failed: {}", e);
                log::warn!("Connection to {} failed: {}", addr, e);
                Err(SshConnError::Connection(error_msg))
            }
            Err(_) => {
                let timeout_secs = timeout_secs.unwrap_or(self.default_timeout);
                let error_msg = format!("Connection timeout after {}s", timeout_secs);
                log::warn!("Connection to {} timed out", addr);
                Err(SshConnError::Connection(error_msg))
            }
        }
    }

    /// 连续ping测试，返回平均延迟
    pub async fn ping_test(&self, hostname: &str, port: u16, count: u32) -> Result<(Duration, Vec<Duration>)> {
        let mut results = Vec::new();
        let mut successful_count = 0;
        let mut total_duration = Duration::ZERO;

        for i in 0..count {
            match self.test_connection(hostname, port, Some(3)).await {
                Ok(duration) => {
                    results.push(duration);
                    total_duration += duration;
                    successful_count += 1;
                    log::debug!("Ping {}/{} to {}:{} - {}ms", i + 1, count, hostname, port, duration.as_millis());
                }
                Err(e) => {
                    log::warn!("Ping {}/{} to {}:{} failed: {}", i + 1, count, hostname, port, e);
                    results.push(Duration::from_millis(u64::MAX)); // 表示失败
                }
            }
            
            // 避免过于频繁的请求
            if i < count - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        if successful_count == 0 {
            return Err(SshConnError::Connection(format!(
                "All {} ping attempts to {}:{} failed", 
                count, hostname, port
            )));
        }

        let average = total_duration / successful_count;
        Ok((average, results))
    }
}

impl Default for NetworkProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConnectionStatus, SshHost};

    #[tokio::test]
    async fn test_probe_creation() {
        let probe = NetworkProbe::new();
        assert_eq!(probe.default_timeout, 5);

        let probe = NetworkProbe::new().with_timeout(10);
        assert_eq!(probe.default_timeout, 10);
    }

    #[tokio::test]
    async fn test_localhost_connection() {
        let probe = NetworkProbe::new();
        
        // 测试本地SSH端口（如果存在）
        match probe.test_connection("127.0.0.1", 22, Some(1)).await {
            Ok(duration) => {
                println!("Local SSH connection successful: {:?}", duration);
            }
            Err(_) => {
                println!("Local SSH connection failed (expected if SSH is not running)");
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_connection() {
        let probe = NetworkProbe::new();
        
        // 测试无效端口
        let result = probe.test_connection("127.0.0.1", 65534, Some(1)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_host_connection() {
        let mut host = SshHost::new("test-host".to_string());
        host.hostname = Some("127.0.0.1".to_string());
        host.port = Some("22".to_string());
        host.connect_timeout = Some("1".to_string());

        let probe = NetworkProbe::new();
        let _ = probe.test_host(&mut host).await;
        
        // 检查状态是否已更新
        assert!(!matches!(host.connection_status, ConnectionStatus::Unknown));
    }
}
