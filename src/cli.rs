//! 命令行接口模块

use clap::{Parser, Subcommand};

use crate::config::ConfigManager;
use crate::error::Result;
use crate::i18n::t;
use crate::ui::UiManager;

/// Command line interface
#[derive(Parser)]
#[command(
    name = "ssh-conn",
    about = "List and connect to SSH servers configured in ssh config",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// List all SSH servers configured in ssh config
    List,
    /// Connect to specified server
    Connect {
        /// Host name in ssh config
        host: String,
    },
    /// Add server to ssh config
    Add {
        /// Host name
        host: String,
        /// Server address (HostName)
        hostname: String,
        /// Username (optional)
        #[arg(short, long)]
        user: Option<String>,
        /// Port (optional)
        #[arg(short, long)]
        port: Option<u16>,
        /// ProxyCommand (optional)
        #[arg(long)]
        proxy_command: Option<String>,
        /// IdentityFile (optional)
        #[arg(long)]
        identity_file: Option<String>,
    },
    /// Edit server configuration
    Edit {
        /// Host name to edit
        host: String,
        /// Server address (HostName, optional)
        #[arg(long)]
        hostname: Option<String>,
        /// Username (optional)
        #[arg(short, long)]
        user: Option<String>,
        /// Port (optional)
        #[arg(short, long)]
        port: Option<u16>,
        /// ProxyCommand (optional)
        #[arg(long)]
        proxy_command: Option<String>,
        /// IdentityFile (optional)
        #[arg(long)]
        identity_file: Option<String>,
    },
    /// Delete server configuration
    Delete {
        /// Host name to delete
        host: String,
    },
    /// Search servers
    Search {
        /// Search query
        query: String,
    },
    /// Backup configuration file
    Backup,
}

/// 命令行应用
pub struct CliApp {
    config_manager: ConfigManager,
}

impl CliApp {
    /// 创建一个新的命令行应用
    pub fn new(config_manager: ConfigManager) -> Self {
        Self { config_manager }
    }

    /// 运行命令行应用
    ///
    /// # 参数
    ///
    /// * `cli` - 解析后的命令行参数
    ///
    /// # 返回值
    ///
    /// 返回操作结果，如果操作失败则返回错误
    pub fn run(&mut self, cli: Cli) -> Result<()> {
        match cli.command {
            // 无参数时进入 TUI
            None => {
                let mut ui_manager = UiManager::new(self.config_manager.clone());
                ui_manager
                    .start_tui()
                    .map_err(crate::error::SshConnError::Io)
            }
            Some(cmd) => self.handle_command(cmd),
        }
    }

    /// 处理具体命令
    fn handle_command(&mut self, cmd: Commands) -> Result<()> {
        match cmd {
            Commands::List => self.list_hosts(),
            Commands::Connect { host } => self.connect_host(host),
            Commands::Add {
                host,
                hostname,
                user,
                port,
                proxy_command,
                identity_file,
            } => self.add_host_command(host, hostname, user, port, proxy_command, identity_file),
            Commands::Edit {
                host,
                hostname,
                user,
                port,
                proxy_command,
                identity_file,
            } => self.edit_host_command(host, hostname, user, port, proxy_command, identity_file),
            Commands::Delete { host } => self.delete_host_command(host),
            Commands::Search { query } => self.search_hosts(&query),
            Commands::Backup => self.backup_config(),
        }
    }

    /// 连接到指定主机
    fn connect_host(&mut self, host: String) -> Result<()> {
        self.config_manager.connect_host(&host)?;
        Ok(())
    }

    /// 列出所有主机
    fn list_hosts(&mut self) -> Result<()> {
        let hosts = self.config_manager.get_hosts()?.clone();

        if hosts.is_empty() {
            println!("{}", t("no_ssh_config_found"));
            return Ok(());
        }

        println!("{}:", t("server_list"));
        println!("{:-<80}", "");

        for host in &hosts {
            println!("{}", self.format_host_info(host));
            println!();
        }

        Ok(())
    }

    /// 搜索主机
    fn search_hosts(&mut self, query: &str) -> Result<()> {
        let hosts = self.config_manager.get_hosts()?.clone();

        let filtered_hosts: Vec<_> = hosts
            .iter()
            .filter(|host| host.matches_query(query))
            .collect();

        if filtered_hosts.is_empty() {
            println!("{}", t("no_matching_servers").replace("{}", query));
            return Ok(());
        }

        println!("{}", t("search_results").replace("{}", query));
        println!("{:-<80}", "");

        for host in &filtered_hosts {
            println!("{}", self.format_host_info(host));
            println!();
        }

        Ok(())
    }

    /// 备份配置
    fn backup_config(&self) -> Result<()> {
        let backup_path = self.config_manager.backup_config()?;
        println!("✓ {}: {}", t("config_backup_success"), backup_path);
        Ok(())
    }

    /// 添加主机命令
    fn add_host_command(
        &mut self,
        host: String,
        hostname: String,
        user: Option<String>,
        port: Option<u16>,
        proxy_command: Option<String>,
        identity_file: Option<String>,
    ) -> Result<()> {
        self.config_manager.add_host(
            &host,
            &hostname,
            user.as_deref(),
            port,
            proxy_command.as_deref(),
            identity_file.as_deref(),
            None, // 命令行模式下不设置密码
        )?;

        println!("✓ {}: {}", t("success_add_server"), host);
        Ok(())
    }

    /// 编辑主机命令
    fn edit_host_command(
        &mut self,
        host: String,
        hostname: Option<String>,
        user: Option<String>,
        port: Option<u16>,
        proxy_command: Option<String>,
        identity_file: Option<String>,
    ) -> Result<()> {
        self.config_manager.edit_host(
            &host,
            hostname.as_deref(),
            user.as_deref(),
            port,
            proxy_command.as_deref(),
            identity_file.as_deref(),
            None, // 命令行模式下不设置密码
        )?;

        println!("✓ {}: {}", t("success_update_server"), host);
        Ok(())
    }

    /// 删除主机命令
    fn delete_host_command(&mut self, host: String) -> Result<()> {
        self.config_manager.delete_host(&host)?;
        println!("✓ {}: {}", t("success_delete_server"), host);
        Ok(())
    }

    /// 格式化主机信息显示
    fn format_host_info(&self, host: &crate::models::SshHost) -> String {
        let mut lines = vec![format!("{}: {}", t("cli_labels.host"), host.host)];

        if let Some(hostname) = &host.hostname {
            lines.push(format!("  {}: {}", t("cli_labels.hostname"), hostname));
        }

        if let Some(user) = &host.user {
            lines.push(format!("  {}: {}", t("cli_labels.user"), user));
        }

        if let Some(port) = &host.port {
            lines.push(format!("  {}: {}", t("cli_labels.port"), port));
        }

        if let Some(proxy) = &host.proxy_command {
            lines.push(format!("  {}: {}", t("cli_labels.proxy_command"), proxy));
        }

        if let Some(identity) = &host.identity_file {
            lines.push(format!("  {}: {}", t("cli_labels.identity_file"), identity));
        }

        lines.join("\n")
    }
}
