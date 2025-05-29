//! 密码管理模块

use crate::error::{Result, SshConnError};
use crate::utils::get_password_db_path;
use rusqlite::{Connection, params};
use std::collections::HashMap;

/// 密码管理器
#[derive(Clone)]
pub struct PasswordManager {
    /// 数据库路径
    db_path: String,
    /// 数据库密码
    db_password: String,
    /// 密码缓存
    password_cache: HashMap<String, String>,
}

impl PasswordManager {
    /// 创建一个新的密码管理器
    pub fn new() -> Result<Self> {
        let db_path = get_password_db_path()?.to_string_lossy().to_string();

        // 初始化密码管理器
        let mut manager = Self {
            db_path,
            db_password: String::new(), // 默认为空密码
            password_cache: HashMap::new(),
        };

        // 加载所有密码到缓存
        manager.load_all_passwords()?;

        Ok(manager)
    }

    /// 设置数据库密码
    pub fn set_db_password(&mut self, password: &str) -> Result<()> {
        self.db_password = password.to_string();
        // 重新加载密码
        self.load_all_passwords()?;
        Ok(())
    }

    /// 打开密码数据库连接
    fn open_db(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path).map_err(SshConnError::Database)?;

        // 如果有设置密码，则使用密码
        if !self.db_password.is_empty() {
            conn.pragma_update(None, "key", &self.db_password)
                .map_err(SshConnError::Database)?;
        }

        // 创建密码表（如果不存在）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS passwords (host TEXT PRIMARY KEY, password TEXT)",
            [],
        )
        .map_err(SshConnError::Database)?;

        Ok(conn)
    }

    /// 保存密码
    pub fn save_password(&mut self, host: &str, password: &str) -> Result<()> {
        // 更新缓存
        self.password_cache
            .insert(host.to_string(), password.to_string());

        // 保存到数据库
        let conn = self.open_db()?;
        conn.execute(
            "INSERT OR REPLACE INTO passwords (host, password) VALUES (?1, ?2)",
            params![host, password],
        )
        .map_err(SshConnError::Database)?;

        Ok(())
    }

    /// 获取密码
    pub fn get_password(&self, host: &str) -> Option<String> {
        // 先从缓存中查找
        if let Some(password) = self.password_cache.get(host) {
            return Some(password.clone());
        }

        // 如果缓存中没有，尝试从数据库加载
        match self.open_db() {
            Ok(conn) => {
                let mut stmt = match conn.prepare("SELECT password FROM passwords WHERE host = ?1")
                {
                    Ok(stmt) => stmt,
                    Err(_) => return None,
                };

                let mut rows = match stmt.query(params![host]) {
                    Ok(rows) => rows,
                    Err(_) => return None,
                };

                if let Ok(Some(row)) = rows.next() {
                    if let Ok(password) = row.get::<_, String>(0) {
                        return Some(password);
                    }
                }

                None
            }
            Err(_) => None,
        }
    }

    /// 删除密码
    pub fn delete_password(&mut self, host: &str) -> Result<()> {
        // 从缓存中删除
        self.password_cache.remove(host);

        // 从数据库中删除
        let conn = self.open_db()?;
        conn.execute("DELETE FROM passwords WHERE host = ?1", params![host])
            .map_err(SshConnError::Database)?;

        Ok(())
    }

    /// 加载所有密码到缓存
    fn load_all_passwords(&mut self) -> Result<()> {
        self.password_cache.clear();

        let conn = match self.open_db() {
            Ok(conn) => conn,
            Err(_) => return Ok(()), // 如果数据库不存在，忽略错误
        };

        let mut stmt = conn
            .prepare("SELECT host, password FROM passwords")
            .map_err(SshConnError::Database)?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(SshConnError::Database)?;

        for (host, password) in rows.flatten() {
            self.password_cache.insert(host, password);
        }

        Ok(())
    }

    /// 获取所有密码
    pub fn get_all_passwords(&self) -> &HashMap<String, String> {
        &self.password_cache
    }
}
