//! 数据模型定义

use crate::i18n::t;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    /// 未检测
    Unknown,
    /// 连接中
    Connecting,
    /// 连接成功
    Connected(Duration), // 包含延迟时间
    /// 连接失败
    Failed(String), // 包含错误信息
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl ConnectionStatus {
    /// 获取状态显示字符串
    pub fn display_string(&self) -> String {
        match self {
            ConnectionStatus::Unknown => "⚪".to_string(),
            ConnectionStatus::Connecting => "🟡".to_string(),
            ConnectionStatus::Connected(duration) => {
                format!("🟢 {}ms", duration.as_millis())
            }
            ConnectionStatus::Failed(_) => "🔴".to_string(),
        }
    }

    /// 获取详细状态字符串
    pub fn detail_string(&self) -> String {
        match self {
            ConnectionStatus::Unknown => t("status.unknown"),
            ConnectionStatus::Connecting => t("status.connecting"),
            ConnectionStatus::Connected(duration) => {
                format!("{} ({}ms)", t("status.connected"), duration.as_millis())
            }
            ConnectionStatus::Failed(error) => {
                format!("{}: {}", t("status.failed"), error)
            }
        }
    }
}

/// SSH主机配置结构体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshHost {
    /// 主机名称（Host字段）
    pub host: String,
    /// 主机地址（HostName字段）
    pub hostname: Option<String>,
    /// 用户名（User字段）
    pub user: Option<String>,
    /// 端口（Port字段）
    pub port: Option<String>,
    /// 代理命令（ProxyCommand字段）
    pub proxy_command: Option<String>,
    /// 身份文件（IdentityFile字段）
    pub identity_file: Option<String>,
    /// 连接超时（ConnectTimeout字段）
    pub connect_timeout: Option<String>,
    /// 服务器存活间隔（ServerAliveInterval字段）
    pub server_alive_interval: Option<String>,
    /// 其他自定义配置
    pub custom_options: std::collections::HashMap<String, String>,
    /// 连接状态（不序列化到配置文件）
    #[serde(skip)]
    pub connection_status: ConnectionStatus,
}

impl SshHost {
    /// 创建一个新的SSH主机配置
    pub fn new(host: String) -> Self {
        Self {
            host,
            hostname: None,
            user: None,
            port: None,
            proxy_command: None,
            identity_file: None,
            connect_timeout: None,
            server_alive_interval: None,
            custom_options: std::collections::HashMap::new(),
            connection_status: ConnectionStatus::default(),
        }
    }

    /// 获取连接字符串
    pub fn get_connection_string(&self) -> String {
        match (&self.user, &self.hostname, &self.port) {
            (Some(user), Some(hostname), Some(port)) => {
                format!("{}@{}:{}", user, hostname, port)
            }
            (Some(user), Some(hostname), None) => {
                format!("{}@{}", user, hostname)
            }
            (None, Some(hostname), Some(port)) => {
                format!("{}:{}", hostname, port)
            }
            (None, Some(hostname), None) => hostname.clone(),
            _ => self.host.clone(),
        }
    }

    /// 检查是否匹配搜索查询
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.to_lowercase();

        self.host.to_lowercase().contains(&query)
            || self
                .hostname
                .as_ref()
                .is_some_and(|h| h.to_lowercase().contains(&query))
            || self
                .user
                .as_ref()
                .is_some_and(|u| u.to_lowercase().contains(&query))
            || self.port.as_ref().is_some_and(|p| p.contains(&query))
    }

    /// 转换为配置文件格式
    pub fn to_config_format(&self) -> String {
        let mut lines = vec![format!("Host {}", self.host)];

        if let Some(hostname) = &self.hostname {
            lines.push(format!("    HostName {}", hostname));
        }

        if let Some(user) = &self.user {
            lines.push(format!("    User {}", user));
        }

        if let Some(port) = &self.port {
            lines.push(format!("    Port {}", port));
        }

        if let Some(proxy_command) = &self.proxy_command {
            lines.push(format!("    ProxyCommand {}", proxy_command));
        }

        if let Some(identity_file) = &self.identity_file {
            lines.push(format!("    IdentityFile {}", identity_file));
        }

        if let Some(connect_timeout) = &self.connect_timeout {
            lines.push(format!("    ConnectTimeout {}", connect_timeout));
        }

        if let Some(server_alive_interval) = &self.server_alive_interval {
            lines.push(format!("    ServerAliveInterval {}", server_alive_interval));
        }

        // 添加自定义选项
        for (key, value) in &self.custom_options {
            lines.push(format!("    {} {}", key, value));
        }

        lines.join("\n")
    }

    /// 获取实际的主机名和端口
    pub fn get_host_and_port(&self) -> (String, u16) {
        let hostname = self.hostname.as_ref().unwrap_or(&self.host).clone();
        let port = self
            .port
            .as_ref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(22);
        (hostname, port)
    }

    /// 异步测试端口连通性
    pub async fn test_connection(&mut self) -> crate::error::Result<()> {
        use tokio::net::TcpStream;
        use tokio::time::{Instant, sleep, timeout};

        // 只有在状态不是Connecting时才设置为Connecting
        // 这样可以避免UI中已经设置的Connecting状态被覆盖
        let connecting_start = Instant::now();
        if !matches!(self.connection_status, ConnectionStatus::Connecting) {
            self.connection_status = ConnectionStatus::Connecting;
        }

        let (hostname, port) = self.get_host_and_port();
        let addr = format!("{}:{}", hostname, port);

        // 获取连接超时时间，默认5秒
        let timeout_secs = self
            .connect_timeout
            .as_ref()
            .and_then(|t| t.parse().ok())
            .unwrap_or(5);

        let start_time = Instant::now();

        let result =
            match timeout(Duration::from_secs(timeout_secs), TcpStream::connect(&addr)).await {
                Ok(Ok(_stream)) => {
                    let duration = start_time.elapsed();
                    self.connection_status = ConnectionStatus::Connected(duration);
                    log::debug!("Connection to {} successful in {:?}", addr, duration);
                    Ok(())
                }
                Ok(Err(e)) => {
                    let error_msg = format!("Connection failed: {}", e);
                    self.connection_status = ConnectionStatus::Failed(error_msg.clone());
                    log::warn!("Connection to {} failed: {}", addr, e);
                    Err(crate::error::SshConnError::Connection(error_msg))
                }
                Err(_) => {
                    let error_msg = format!("Connection timeout after {}s", timeout_secs);
                    self.connection_status = ConnectionStatus::Failed(error_msg.clone());
                    log::warn!("Connection to {} timed out", addr);
                    Err(crate::error::SshConnError::Connection(error_msg))
                }
            };

        // 确保Connecting状态至少显示200ms，这样用户能看到🟡状态
        let elapsed = connecting_start.elapsed();
        if elapsed < Duration::from_millis(200) {
            sleep(Duration::from_millis(200) - elapsed).await;
        }

        result
    }
}

/// 表单字段定义
#[derive(Debug, Clone)]
pub struct FormField {
    /// 字段标签
    pub label: String,
    /// 字段值
    pub value: String,
    /// 是否必填
    pub required: bool,
    /// 字段类型
    pub field_type: FormFieldType,
    /// 是否只读
    pub readonly: bool,
}

/// 表单字段类型
#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldType {
    Text,
    Number,
    Password,
    Path,
}

impl FormField {
    /// 创建一个新的表单字段
    pub fn new<S1: Into<String>, S2: Into<String>>(label: S1, value: S2) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            required: false,
            field_type: FormFieldType::Text,
            readonly: false,
        }
    }

    /// 创建必填字段
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置字段类型
    pub fn with_type(mut self, field_type: FormFieldType) -> Self {
        self.field_type = field_type;
        self
    }

    /// 设置为只读字段
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// 验证字段值
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.required && self.value.is_empty() {
            return Err(crate::error::SshConnError::ConfigParse(
                t("field_required").replace("{}", &self.label),
            ));
        }

        match self.field_type {
            FormFieldType::Number => {
                if !self.value.is_empty() {
                    crate::utils::validate_port(&self.value)?;
                }
            }
            FormFieldType::Path => {
                if !self.value.is_empty() {
                    let path = std::path::Path::new(&self.value);
                    if !path.exists() {
                        log::warn!("{}", t("path_not_exists").replace("{}", &self.value));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
