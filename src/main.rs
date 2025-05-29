use clap::Parser;
use std::process;

use ssh_conn::cli::{Cli, CliApp};
use ssh_conn::config::ConfigManager;
use ssh_conn::error::Result;
use ssh_conn::i18n::t;
use ssh_conn::password::PasswordManager;

fn main() {
    // 初始化日志系统
    env_logger::init();

    if let Err(e) = run() {
        eprintln!("{}: {}", t("error"), e.localized_message());
        process::exit(1);
    }
}

/// 主运行函数
///
/// 初始化所有组件并运行命令行应用
fn run() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 初始化密码管理器
    let password_manager = PasswordManager::new()?;

    // 初始化配置管理器
    let config_manager = ConfigManager::new(password_manager)?;

    // 创建并运行命令行应用
    let mut app = CliApp::new(config_manager);
    app.run(cli)
}
