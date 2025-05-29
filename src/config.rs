//! SSH配置文件管理模块

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::error::{Result, SshConnError};
use crate::i18n::t;
use crate::models::SshHost;
use crate::password::PasswordManager;
use crate::utils::*;

/// 通用SSH连接参数
const DEFAULT_SSH_OPTIONS: &[&str] = &[
    "-o", "StrictHostKeyChecking=accept-new",
    "-o", "LogLevel=ERROR",
];

/// TUI模式的SSH连接参数
const TUI_SSH_OPTIONS: &[&str] = &[
    "-o", "StrictHostKeyChecking=accept-new",
    "-o", "LogLevel=ERROR",
    "-o", "RequestTTY=force",
    "-tt",
];

/// 连接测试的SSH参数
const TEST_SSH_OPTIONS: &[&str] = &[
    "-o", "ConnectTimeout=10",
    "-o", "StrictHostKeyChecking=yes",
];

/// 写入SSH配置选项的辅助函数
fn write_ssh_option<W: Write>(
    file: &mut W,
    key: &str,
    new_value: Option<&str>,
    original_value: Option<&str>,
) -> Result<()> {
    if let Some(value) = new_value {
        writeln!(file, "    {} {}", key, value)?;
    } else if let Some(value) = original_value {
        writeln!(file, "    {} {}", key, value)?;
    }
    Ok(())
}

/// SSH配置管理器
#[derive(Clone)]
pub struct ConfigManager {
    config_path: String,
    password_manager: PasswordManager,
    /// 缓存的主机配置
    hosts_cache: Option<Vec<SshHost>>,
}

/// 跨平台执行命令的辅助函数
/// 在Unix系统上使用exec()替换当前进程，在Windows上使用spawn()并等待
#[cfg(unix)]
fn exec_command(mut cmd: std::process::Command) -> Result<()> {
    let result = cmd.exec();
    Err(SshConnError::SshConnectionError(
        format!("Command exec failed: {:?}", result)
    ))
}

#[cfg(windows)]
fn exec_command(mut cmd: std::process::Command) -> Result<()> {
    let status = cmd.status().map_err(|e| {
        SshConnError::SshConnectionError(
            format!("Command execution failed: {}", e)
        )
    })?;
    
    if status.success() {
        // 在Windows上，我们不能真正替换进程，所以这里成功退出
        std::process::exit(0);
    } else {
        let code = status.code().unwrap_or(-1);
        Err(SshConnError::SshConnectionError(
            format!("Command failed with code: {}", code)
        ))
    }
}

impl ConfigManager {
    /// 创建一个新的配置管理器
    pub fn new(password_manager: PasswordManager) -> Result<Self> {
        let config_path = get_ssh_config_path()?.to_string_lossy().to_string();

        Ok(Self {
            config_path,
            password_manager,
            hosts_cache: None,
        })
    }

    /// 获取所有主机配置
    pub fn get_hosts(&mut self) -> Result<&Vec<SshHost>> {
        // 如果缓存存在，直接返回缓存
        if let Some(ref hosts) = self.hosts_cache {
            return Ok(hosts);
        }

        // 否则解析配置文件
        let hosts = self.parse_ssh_config()?;
        self.hosts_cache = Some(hosts);
        Ok(self.hosts_cache.as_ref().unwrap())
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.hosts_cache = None;
    }

    /// 解析SSH配置文件
    fn parse_ssh_config(&self) -> Result<Vec<SshHost>> {
        let file = match File::open(&self.config_path) {
            Ok(file) => file,
            Err(_) => {
                // 如果配置文件不存在，返回空列表
                return Ok(Vec::new());
            }
        };

        let reader = BufReader::new(file);
        let mut hosts = Vec::new();
        let mut current: Option<SshHost> = None;

        for line_result in reader.lines() {
            let line = line_result?;
            let line = line.trim();

            if line.starts_with("Host ") && !line.starts_with("HostName") {
                if let Some(h) = current.take() {
                    hosts.push(h);
                }

                for h in line[5..].split_whitespace() {
                    if h != "*" {
                        // 忽略通配符主机
                        current = Some(SshHost::new(h.to_string()));
                        break; // 只取第一个非通配符主机
                    }
                }
            } else if let Some(ref mut h) = current {
                if let Some(stripped) = line.strip_prefix("HostName ") {
                    h.hostname = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("User ") {
                    h.user = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("Port ") {
                    h.port = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("ProxyCommand ") {
                    h.proxy_command = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("IdentityFile ") {
                    h.identity_file = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("ConnectTimeout ") {
                    h.connect_timeout = Some(stripped.trim().to_string());
                } else if let Some(stripped) = line.strip_prefix("ServerAliveInterval ") {
                    h.server_alive_interval = Some(stripped.trim().to_string());
                } else {
                    // 处理其他自定义选项
                    if let Some(space_pos) = line.find(' ') {
                        let key = line[..space_pos].trim().to_string();
                        let value = line[space_pos + 1..].trim().to_string();
                        if !key.is_empty() && !value.is_empty() {
                            h.custom_options.insert(key, value);
                        }
                    }
                }
            }
        }

        if let Some(h) = current {
            hosts.push(h);
        }

        Ok(hosts)
    }

    /// 列出所有主机
    pub fn list_hosts(&mut self) -> Result<Vec<String>> {
        let hosts = self.get_hosts()?;
        Ok(hosts.iter().map(|h| h.host.clone()).collect())
    }

    /// 添加主机
    #[allow(clippy::too_many_arguments)]
    pub fn add_host(
        &mut self,
        host: &str,
        hostname: &str,
        user: Option<&str>,
        port: Option<u16>,
        proxy_command: Option<&str>,
        identity_file: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        // 验证输入
        validate_host(host)?;
        validate_hostname(hostname)?;

        if let Some(p) = port {
            validate_port(&p.to_string())?;
        }

        // 检查主机名是否已存在
        if self.host_exists(host)? {
            return Err(SshConnError::HostAlreadyExists {
                host: host.to_string(),
            });
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config_path)?;

        writeln!(file, "\nHost {}", host)?;
        writeln!(file, "    HostName {}", hostname)?;

        if let Some(user) = user {
            writeln!(file, "    User {}", user)?;
        }

        if let Some(port) = port {
            writeln!(file, "    Port {}", port)?;
        }

        if let Some(proxy_command) = proxy_command {
            writeln!(file, "    ProxyCommand {}", proxy_command)?;
        }

        if let Some(identity_file) = identity_file {
            writeln!(file, "    IdentityFile {}", identity_file)?;
        }

        // 如果提供了密码，保存到密码管理器
        if let Some(password) = password {
            if !password.is_empty() {
                self.password_manager.save_password(host, password)?;
            }
        }

        // 清除缓存
        self.clear_cache();

        log::info!("{}: {}", t("log_success_add_host"), host);
        Ok(())
    }

    /// 编辑主机
    #[allow(clippy::too_many_arguments)]
    pub fn edit_host(
        &mut self,
        host: &str,
        hostname: Option<&str>,
        user: Option<&str>,
        port: Option<u16>,
        proxy_command: Option<&str>,
        identity_file: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        // 验证输入
        validate_host(host)?;

        if let Some(h) = hostname {
            validate_hostname(h)?;
        }

        if let Some(p) = port {
            validate_port(&p.to_string())?;
        }

        // 获取当前主机列表并保存原始配置
        let original_host = {
            let hosts = self.get_hosts()?;

            // 检查主机是否存在
            if !hosts.iter().any(|h| h.host == host) {
                return Err(SshConnError::HostNotFound {
                    host: host.to_string(),
                });
            }

            // 保存原始主机配置
            hosts.iter().find(|h| h.host == host).cloned()
        };

        // 使用更简洁的方法：删除旧的配置，添加新的配置
        self.delete_host_internal(host)?;

        // 重新添加主机配置
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config_path)?;

        writeln!(file, "\nHost {}", host)?;

        // 使用辅助函数简化代码
        write_ssh_option(
            &mut file,
            "HostName",
            hostname,
            original_host.as_ref().and_then(|o| o.hostname.as_deref()),
        )?;

        write_ssh_option(
            &mut file,
            "User",
            user,
            original_host.as_ref().and_then(|o| o.user.as_deref()),
        )?;

        write_ssh_option(
            &mut file,
            "Port",
            port.map(|p| p.to_string()).as_deref(),
            original_host.as_ref().and_then(|o| o.port.as_deref()),
        )?;

        write_ssh_option(
            &mut file,
            "ProxyCommand",
            proxy_command,
            original_host.as_ref().and_then(|o| o.proxy_command.as_deref()),
        )?;

        write_ssh_option(
            &mut file,
            "IdentityFile",
            identity_file,
            original_host.as_ref().and_then(|o| o.identity_file.as_deref()),
        )?;

        // 如果提供了密码，保存到密码管理器
        if let Some(password) = password {
            if !password.is_empty() {
                self.password_manager.save_password(host, password)?;
            }
        }

        // 清除缓存
        self.clear_cache();

        log::info!("{}: {}", t("log_success_edit_host"), host);
        Ok(())
    }

    /// 删除主机（内部方法，不删除密码）
    fn delete_host_internal(&mut self, host: &str) -> Result<()> {
        let content = std::fs::read_to_string(&self.config_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut new_content = String::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("Host ") && !trimmed.starts_with("HostName") {
                let hosts_in_line: Vec<&str> = trimmed[5..].split_whitespace().collect();

                if hosts_in_line.contains(&host) {
                    // 跳过这个Host块的所有行
                    i += 1;
                    while i < lines.len() {
                        let next_line = lines[i].trim();
                        if next_line.starts_with("Host ") && !next_line.starts_with("HostName") {
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
            }

            new_content.push_str(line);
            new_content.push('\n');
            i += 1;
        }

        std::fs::write(&self.config_path, new_content)?;
        Ok(())
    }

    /// 删除主机
    pub fn delete_host(&mut self, host: &str) -> Result<()> {
        validate_host(host)?;

        // 检查主机是否存在
        if !self.host_exists(host)? {
            return Err(SshConnError::HostNotFound {
                host: host.to_string(),
            });
        }

        self.delete_host_internal(host)?;

        // 删除密码
        self.password_manager.delete_password(host)?;

        // 清除缓存
        self.clear_cache();

        log::info!("{}: {}", t("log_success_delete_host"), host);
        Ok(())
    }
    /// 连接到主机
    pub fn connect_host(&self, host: &str) -> Result<()> {
        validate_host(host)?;

        log::info!("{}: {}", t("log_connecting_to_host"), host);

        // 显示连接信息
        println!("{}: {}", t("connecting_to_host"), host);

        self.connect_host_internal(host)
    }

    /// 内部SSH连接方法
    fn connect_host_internal(&self, host: &str) -> Result<()> {
        self.execute_ssh_connection(host, true, DEFAULT_SSH_OPTIONS, false)
    }

    /// 执行SSH连接的辅助方法
    fn execute_ssh_connection(
        &self,
        host: &str,
        use_password: bool,
        additional_options: &[&str],
        use_exec: bool,
    ) -> Result<()> {
        let password = if use_password {
            self.password_manager.get_password(host)
        } else {
            None
        };

        match password {
            Some(password) if !password.is_empty() => {
                log::info!("{}", t("using_stored_password_auto_login"));
                if !use_exec {
                    println!("{}", t("using_stored_password"));
                }

                let mut cmd = std::process::Command::new("sshpass");
                cmd.arg("-p").arg(&password).arg("ssh");
                
                for option in additional_options {
                    cmd.arg(option);
                }
                cmd.arg(host);

                if use_exec {
                    return exec_command(cmd);
                } else {
                    let status = cmd.status().map_err(|e| {
                        SshConnError::SshConnectionError(
                            t("sshpass_not_available").replace("{}", &e.to_string()),
                        )
                    })?;

                    if let Some(code) = status.code() {
                        if code == 255 {
                            return Err(SshConnError::SshConnectionError(format!(
                                "{}: {}",
                                t("ssh_connection_failed_code"),
                                code
                            )));
                        }
                    }
                }
            }
            _ => {
                log::info!("{}", t("using_ssh_key_auth"));
                if !use_exec {
                    println!("{}", t("using_ssh_key_or_manual"));
                }

                let mut cmd = std::process::Command::new("ssh");
                for option in additional_options {
                    cmd.arg(option);
                }
                cmd.arg(host);

                if use_exec {
                    return exec_command(cmd);
                } else {
                    let status = cmd.status().map_err(|e| {
                        SshConnError::SshConnectionError(
                            t("ssh_start_failed").replace("{}", &e.to_string()),
                        )
                    })?;

                    if let Some(code) = status.code() {
                        if code == 255 {
                            return Err(SshConnError::SshConnectionError(format!(
                                "{}: {}",
                                t("ssh_connection_failed_code"),
                                code
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 检测主机密钥验证失败
    fn is_host_key_verification_failed(stderr: &str) -> bool {
        stderr.contains("Host key verification failed")
            || stderr.contains("REMOTE HOST IDENTIFICATION HAS CHANGED")
            || stderr.contains("Someone could be eavesdropping on you right now")
            || (stderr.contains("Host key for") && stderr.contains("has changed"))
    }

    /// 处理主机密钥验证失败（TUI专用方法）
    /// 使用与TUI连接一致的方式，确保能够正常返回界面
    pub fn handle_host_key_verification_failed_for_tui(&self, host: &str) -> Result<()> {
        log::info!("{}", t("tui_mode_host_key_failed"));

        // 从known_hosts中移除旧的主机密钥
        let status = std::process::Command::new("ssh-keygen")
            .arg("-R")
            .arg(host)
            .status()
            .map_err(|e| {
                SshConnError::SshConnectionError(
                    t("ssh_keygen_exec_failed").replace("{}", &e.to_string()),
                )
            })?;

        if !status.success() {
            log::warn!("{}", t("ssh_keygen_failed_continue"));
        }

        // 重新尝试连接，这次接受新的主机密钥，并自动带入存储的密码
        println!("{}", t("reconnecting_accept_key"));

        // 检查是否有存储的密码
        match self.password_manager.get_password(host) {
            Some(password) => {
                log::info!("{}", t("log_using_stored_password_reconnect"));
                println!("{}", t("using_stored_password"));

                // 使用 sshpass 和存储的密码，保存主机密钥到known_hosts
                let status = std::process::Command::new("sshpass")
                    .arg("-p")
                    .arg(&password)
                    .arg("ssh")
                    .args(TUI_SSH_OPTIONS)
                    .arg(host)
                    .status()
                    .map_err(|e| {
                        SshConnError::SshConnectionError(
                            t("sshpass_not_available_simple").replace("{}", &e.to_string()),
                        )
                    })?;

                // 使用与TUI连接一致的错误处理逻辑
                if let Some(code) = status.code() {
                    if code == 255 {
                        return Err(SshConnError::SshConnectionError(format!(
                            "{}: {}",
                            t("ssh_connection_failed_code"),
                            code
                        )));
                    }
                    // 其他退出码（如1,2等）通常表示用户正常退出或远程命令执行结果，不是连接错误
                }
            }
            None => {
                log::info!("{}", t("log_no_stored_password_use_ssh"));
                println!("{}", t("using_ssh_key_or_manual"));

                // 使用普通 SSH 连接，保存主机密钥到known_hosts
                let status = std::process::Command::new("ssh")
                    .args(TUI_SSH_OPTIONS)
                    .arg(host)
                    .status()
                    .map_err(|e| {
                        SshConnError::SshConnectionError(
                            t("ssh_start_failed").replace("{}", &e.to_string()),
                        )
                    })?;

                // 使用与TUI连接一致的错误处理逻辑
                if let Some(code) = status.code() {
                    if code == 255 {
                        return Err(SshConnError::SshConnectionError(format!(
                            "{}: {}",
                            t("ssh_connection_failed_code"),
                            code
                        )));
                    }
                    // 其他退出码（如1,2等）通常表示用户正常退出或远程命令执行结果，不是连接错误
                }
            }
        }

        Ok(())
    }

    /// 处理主机密钥验证失败（非交互模式，用于CLI）
    pub fn handle_host_key_verification_failed_non_interactive(&self, host: &str) -> Result<()> {
        log::info!("{}", t("non_interactive_mode_host_key_failed"));

        // 从known_hosts中移除旧的主机密钥
        let status = std::process::Command::new("ssh-keygen")
            .arg("-R")
            .arg(host)
            .status()
            .map_err(|e| {
                SshConnError::SshConnectionError(
                    t("ssh_keygen_exec_failed").replace("{}", &e.to_string()),
                )
            })?;

        if !status.success() {
            log::warn!("{}", t("ssh_keygen_failed_continue"));
        }

        // 重新尝试连接，这次接受新的主机密钥，并自动带入存储的密码
        println!("{}", t("reconnecting_accept_key"));

        // 检查是否有存储的密码
        match self.password_manager.get_password(host) {
            Some(password) => {
                log::info!("{}", t("log_using_stored_password_reconnect"));
                println!("{}", t("using_stored_password"));

                // CLI模式使用 exec，替换当前进程，保存主机密钥到known_hosts
                let mut cmd = std::process::Command::new("sshpass");
                cmd.arg("-p")
                   .arg(&password)
                   .arg("ssh")
                   .args(DEFAULT_SSH_OPTIONS)
                   .arg(host);
                
                return exec_command(cmd);
            }
            None => {
                log::info!("{}", t("log_no_stored_password_use_ssh"));
                println!("{}", t("using_ssh_key_or_manual"));

                // CLI模式使用 exec，替换当前进程
                let mut cmd = std::process::Command::new("ssh");
                cmd.args(DEFAULT_SSH_OPTIONS)
                   .arg(host);
                
                return exec_command(cmd);
            }
        }
    }

    /// 尝试连接主机并检测主机密钥验证失败（用于TUI模式）
    /// 返回 (success, host_key_error, error_message)
    pub fn try_connect_host(&self, host: &str) -> (bool, bool, Option<String>) {
        let _ssh_host = match self
            .hosts_cache
            .as_ref()
            .and_then(|hosts| hosts.iter().find(|h| h.host == host))
        {
            Some(host) => host,
            None => return (false, false, Some(t("host_not_exists"))),
        };

        // 首先尝试使用密码连接（如果有密码）
        if let Some(password) = self.password_manager.get_password(host) {
            if !password.is_empty() {
                let output = std::process::Command::new("sshpass")
                    .arg("-p")
                    .arg(&password)
                    .arg("ssh")
                    .args(TEST_SSH_OPTIONS)
                    .arg(host)
                    .arg("exit")
                    .output();

                match output {
                    Ok(result) => {
                        if result.status.success() {
                            return (true, false, None);
                        } else {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            if Self::is_host_key_verification_failed(&stderr) {
                                return (false, true, Some(stderr.to_string()));
                            }
                        }
                    }
                    Err(_) => {
                        // sshpass 不可用，继续尝试普通 SSH
                    }
                }
            }
        }

        // 尝试普通SSH连接
        let output = std::process::Command::new("ssh")
            .args(TEST_SSH_OPTIONS)
            .arg(host)
            .arg("exit")
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    (true, false, None)
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    if Self::is_host_key_verification_failed(&stderr) {
                        (false, true, Some(stderr.to_string()))
                    } else {
                        (false, false, Some(stderr.to_string()))
                    }
                }
            }
            Err(e) => (
                false,
                false,
                Some(format!("{}: {}", t("connection_failed_code"), e)),
            ),
        }
    }

    /// 获取主机详细信息
    pub fn get_host(&mut self, host: &str) -> Result<Option<SshHost>> {
        let hosts = self.get_hosts()?;
        Ok(hosts.iter().find(|h| h.host == host).cloned())
    }

    /// 备份配置文件
    pub fn backup_config(&self) -> Result<String> {
        let backup_path = format!(
            "{}.backup.{}",
            self.config_path,
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );

        std::fs::copy(&self.config_path, &backup_path)?;
        log::info!("{}", t("backup_created_at").replace("{}", &backup_path));

        Ok(backup_path)
    }

    /// 检查主机是否存在于配置中
    pub fn host_exists(&mut self, host: &str) -> Result<bool> {
        let hosts = self.get_hosts()?;
        Ok(hosts.iter().any(|h| h.host == host))
    }

    /// 搜索主机配置
    pub fn search_hosts(&mut self, query: &str) -> Result<Vec<SshHost>> {
        let hosts = self.get_hosts()?;
        let query = query.to_lowercase();
        Ok(hosts
            .iter()
            .filter(|host| {
                host.host.to_lowercase().contains(&query)
                    || host
                        .hostname
                        .as_ref()
                        .map(|h| h.to_lowercase().contains(&query))
                        .unwrap_or(false)
                    || host
                        .user
                        .as_ref()
                        .map(|u| u.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    /// 不使用密码连接主机（仅测试连接）
    pub fn connect_host_without_password(&self, host: &str) -> Result<bool> {
        use std::process::Command;

        // 使用 SSH 的 ConnectTimeout 和 BatchMode 来快速测试连接
        let output = Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=5",
                "-o",
                "BatchMode=yes",
                "-o",
                "PasswordAuthentication=no",
                "-o",
                "PubkeyAuthentication=yes",
                "-o",
                "StrictHostKeyChecking=no",
                host,
                "exit",
            ])
            .output()
            .map_err(|e| {
                SshConnError::SshConnectionError(format!("Failed to execute ssh command: {}", e))
            })?;

        // 如果退出码为 0，说明连接成功（有密钥认证）
        Ok(output.status.success())
    }

    /// 为TUI模式提供的简化连接方法
    /// 直接执行SSH连接，优化终端显示效果
    pub fn connect_host_for_tui(&self, host: &str) -> Result<()> {
        validate_host(host)?;

        log::info!("{}: {}", t("log_tui_connecting_to_host"), host);

        self.execute_ssh_connection(host, true, TUI_SSH_OPTIONS, false)
    }
}
