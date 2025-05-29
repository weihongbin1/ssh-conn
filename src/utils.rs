//! 工具函数模块

use crate::error::{Result, SshConnError};
use crate::i18n::t;
use std::path::PathBuf;

/// 获取SSH配置文件路径
pub fn get_ssh_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| SshConnError::ConfigParse(t("error_home_dir").to_string()))?;

    let ssh_dir = home_dir.join(".ssh");
    if !ssh_dir.exists() {
        std::fs::create_dir_all(&ssh_dir)?;
    }

    Ok(ssh_dir.join("config"))
}

/// 获取密码数据库路径
pub fn get_password_db_path() -> Result<PathBuf> {
    use crate::i18n::t;
    let home_dir = dirs::home_dir()
        .ok_or_else(|| SshConnError::ConfigParse(t("error_home_dir").to_string()))?;

    let ssh_dir = home_dir.join(".ssh");
    if !ssh_dir.exists() {
        std::fs::create_dir_all(&ssh_dir)?;
    }

    Ok(ssh_dir.join("ssh_conn_passwords.db"))
}

/// 验证端口号
pub fn validate_port(port_str: &str) -> Result<u16> {
    if port_str.is_empty() {
        return Err(SshConnError::InvalidPort {
            port: port_str.to_string(),
        });
    }

    let port = port_str
        .parse::<u16>()
        .map_err(|_| SshConnError::InvalidPort {
            port: port_str.to_string(),
        })?;

    if port == 0 {
        return Err(SshConnError::InvalidPort {
            port: port_str.to_string(),
        });
    }

    Ok(port)
}

/// 验证SSH主机名称
pub fn validate_hostname(hostname: &str) -> Result<()> {
    use crate::i18n::t;

    if hostname.is_empty() {
        return Err(SshConnError::ConfigParse(t("validation.hostname_empty")));
    }

    if hostname.trim() != hostname {
        return Err(SshConnError::ConfigParse(t(
            "validation.hostname_whitespace",
        )));
    }

    if hostname.contains(' ') {
        return Err(SshConnError::ConfigParse(t("validation.hostname_spaces")));
    }

    // 检查连续的点号
    if hostname.contains("..") {
        return Err(SshConnError::ConfigParse(t(
            "validation.hostname_consecutive_dots",
        )));
    }

    // 检查以点号开始或结束
    if hostname.starts_with('.') || hostname.ends_with('.') {
        return Err(SshConnError::ConfigParse(t(
            "validation.hostname_starts_or_ends_with_dot",
        )));
    }

    Ok(())
}

/// 验证SSH配置Host字段
pub fn validate_host(host: &str) -> Result<()> {
    if host.is_empty() {
        return Err(SshConnError::ConfigParse(t("host_name_empty").to_string()));
    }

    if host.contains(' ') || host.contains('\t') {
        return Err(SshConnError::ConfigParse(
            t("host_name_no_spaces").to_string(),
        ));
    }

    // 检查是否包含通配符（在某些情况下可能不合适）
    if host.contains('*') || host.contains('?') {
        log::warn!("{}", t("host_name_wildcard_warning"));
    }

    Ok(())
}

/// 验证用户名
pub fn validate_username(username: &str) -> Result<()> {
    if username.is_empty() {
        return Err(SshConnError::ConfigParse(t("username_empty").to_string()));
    }

    if username.contains(' ') || username.contains('\t') {
        return Err(SshConnError::ConfigParse(
            t("username_no_spaces").to_string(),
        ));
    }

    // 检查是否包含非法字符
    if username.contains('@') || username.contains(':') {
        return Err(SshConnError::ConfigParse(
            t("username_invalid_chars").to_string(),
        ));
    }

    Ok(())
}

/// 格式化SSH连接信息用于显示
pub fn format_ssh_info(host: &crate::models::SshHost) -> String {
    let mut info = vec![format!("Host: {}", host.host)];

    if let Some(hostname) = &host.hostname {
        info.push(format!("HostName: {}", hostname));
    }

    if let Some(user) = &host.user {
        info.push(format!("User: {}", user));
    }

    if let Some(port) = &host.port {
        info.push(format!("Port: {}", port));
    }

    if let Some(proxy) = &host.proxy_command {
        info.push(format!("ProxyCommand: {}", proxy));
    }

    if let Some(identity) = &host.identity_file {
        info.push(format!("IdentityFile: {}", identity));
    }

    info.join(", ")
}
