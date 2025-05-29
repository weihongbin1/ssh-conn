//! 错误处理模块

use std::fmt;
use std::io;

/// 应用程序错误类型
#[derive(Debug)]
pub enum SshConnError {
    Io(io::Error),
    Database(rusqlite::Error),
    ConfigParse(String),
    HostNotFound { host: String },
    HostAlreadyExists { host: String },
    InvalidPort { port: String },
    PasswordError(String),
    SshConnectionError(String),
    TuiError(String),
    Connection(String),
}

impl fmt::Display for SshConnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.localized_message())
    }
}

impl std::error::Error for SshConnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SshConnError::Io(err) => Some(err),
            SshConnError::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for SshConnError {
    fn from(err: io::Error) -> Self {
        SshConnError::Io(err)
    }
}

impl From<rusqlite::Error> for SshConnError {
    fn from(err: rusqlite::Error) -> Self {
        SshConnError::Database(err)
    }
}

impl SshConnError {
    /// 获取本地化的错误消息
    pub fn localized_message(&self) -> String {
        use crate::i18n::t;

        match self {
            SshConnError::Io(err) => format!("{}: {}", t("error_io"), err),
            SshConnError::Database(err) => format!("{}: {}", t("error_database"), err),
            SshConnError::ConfigParse(msg) => format!("{}: {}", t("error_config_parse"), msg),
            SshConnError::HostNotFound { host } => {
                format!("{}: '{}'", t("error_host_not_found"), host)
            }
            SshConnError::HostAlreadyExists { host } => {
                format!("{}: '{}'", t("error_host_exists"), host)
            }
            SshConnError::InvalidPort { port } => format!("{}: {}", t("error_invalid_port"), port),
            SshConnError::PasswordError(msg) => format!("{}: {}", t("error_password"), msg),
            SshConnError::SshConnectionError(msg) => {
                format!("{}: {}", t("error_ssh_connection"), msg)
            }
            SshConnError::TuiError(msg) => format!("{}: {}", t("error_tui"), msg),
            SshConnError::Connection(msg) => format!("{}: {}", t("error_connection"), msg),
        }
    }
}

/// 应用程序结果类型
pub type Result<T> = std::result::Result<T, SshConnError>;

impl From<SshConnError> for io::Error {
    fn from(err: SshConnError) -> Self {
        match err {
            SshConnError::Io(io_err) => io_err,
            _ => io::Error::other(err),
        }
    }
}
