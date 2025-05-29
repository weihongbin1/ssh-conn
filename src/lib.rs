//! SSH连接管理工具库

pub mod cli;
pub mod config;
pub mod error;
pub mod i18n;
pub mod models;
pub mod network;
pub mod password;
pub mod ui;
pub mod utils;



// 重新导出常用类型
pub use error::{Result, SshConnError};
pub use models::SshHost;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::t;
    use models::{FormField, FormFieldType, SshHost};

    #[test]
    fn test_ssh_host_new() {
        let host = SshHost::new("test-server".to_string());
        assert_eq!(host.host, "test-server");
        assert_eq!(host.hostname, None);
        assert_eq!(host.user, None);
        assert_eq!(host.port, None);
        assert!(host.custom_options.is_empty());
    }

    #[test]
    fn test_ssh_host_connection_string() {
        let mut host = SshHost::new("test-server".to_string());

        // 测试只有主机名的情况
        assert_eq!(host.get_connection_string(), "test-server");

        // 测试有用户名和主机名的情况
        host.user = Some("user".to_string());
        host.hostname = Some("192.168.1.100".to_string());
        assert_eq!(host.get_connection_string(), "user@192.168.1.100");

        // 测试完整配置的情况
        host.port = Some("2222".to_string());
        assert_eq!(host.get_connection_string(), "user@192.168.1.100:2222");

        // 测试只有主机名和端口的情况
        host.user = None;
        assert_eq!(host.get_connection_string(), "192.168.1.100:2222");
    }

    #[test]
    fn test_ssh_host_matches_query() {
        let mut host = SshHost::new("web-server".to_string());
        host.hostname = Some("example.com".to_string());
        host.user = Some("admin".to_string());
        host.port = Some("22".to_string());

        // 测试匹配主机名
        assert!(host.matches_query("web"));
        assert!(host.matches_query("WEB")); // 大小写不敏感

        // 测试匹配hostname
        assert!(host.matches_query("example"));
        assert!(host.matches_query("EXAMPLE.COM"));

        // 测试匹配用户名
        assert!(host.matches_query("admin"));

        // 测试匹配端口
        assert!(host.matches_query("22"));

        // 测试不匹配
        assert!(!host.matches_query("nonexistent"));
    }

    #[test]
    fn test_ssh_host_to_config_format() {
        let mut host = SshHost::new("test-server".to_string());
        host.hostname = Some("192.168.1.100".to_string());
        host.user = Some("testuser".to_string());
        host.port = Some("2222".to_string());
        host.identity_file = Some("~/.ssh/id_rsa".to_string());

        let config = host.to_config_format();
        let expected_lines = vec![
            "Host test-server",
            "    HostName 192.168.1.100",
            "    User testuser",
            "    Port 2222",
            "    IdentityFile ~/.ssh/id_rsa",
        ];

        for line in expected_lines {
            assert!(
                config.contains(line),
                "{}: {}",
                t("config_format_should_contain").replace("{}", line),
                line
            );
        }
    }

    #[test]
    fn test_ssh_host_with_custom_options() {
        let mut host = SshHost::new("custom-server".to_string());
        host.custom_options
            .insert("StrictHostKeyChecking".to_string(), "no".to_string());
        host.custom_options
            .insert("UserKnownHostsFile".to_string(), "/dev/null".to_string());

        let config = host.to_config_format();
        assert!(config.contains("StrictHostKeyChecking no"));
        assert!(config.contains("UserKnownHostsFile /dev/null"));
    }

    #[test]
    fn test_form_field_new() {
        let field = FormField::new("主机名", "example.com");
        assert_eq!(field.label, "主机名");
        assert_eq!(field.value, "example.com");
        assert!(!field.required);
        assert_eq!(field.field_type, FormFieldType::Text);
    }

    #[test]
    fn test_form_field_required() {
        let field = FormField::new("主机名", "")
            .required()
            .with_type(FormFieldType::Text);

        assert!(field.required);
        assert_eq!(field.field_type, FormFieldType::Text);
    }

    #[test]
    fn test_form_field_validation() {
        // 测试必填字段验证
        let required_field = FormField::new("主机名", "").required();
        assert!(required_field.validate().is_err());

        let filled_required_field = FormField::new("主机名", "example.com").required();
        assert!(filled_required_field.validate().is_ok());

        // 测试非必填空字段
        let optional_field = FormField::new("描述", "");
        assert!(optional_field.validate().is_ok());
    }

    #[test]
    fn test_form_field_number_validation() {
        // 测试有效端口号
        let valid_port = FormField::new("端口", "22").with_type(FormFieldType::Number);
        assert!(valid_port.validate().is_ok());

        // 测试空端口号（应该允许）
        let empty_port = FormField::new("端口", "").with_type(FormFieldType::Number);
        assert!(empty_port.validate().is_ok());
    }

    #[test]
    fn test_ssh_host_serialization() {
        let mut host = SshHost::new("test-server".to_string());
        host.hostname = Some("192.168.1.100".to_string());
        host.user = Some("testuser".to_string());
        host.port = Some("22".to_string());

        // 测试序列化
        let json = serde_json::to_string(&host).unwrap_or_else(|_| panic!("{}", t("serialization_failed")));
        assert!(json.contains("test-server"));
        assert!(json.contains("192.168.1.100"));

        // 测试反序列化
        let deserialized: SshHost =
            serde_json::from_str(&json).unwrap_or_else(|_| panic!("{}", t("deserialization_failed")));
        assert_eq!(host, deserialized);
    }

    #[test]
    fn test_ssh_host_clone() {
        let mut host = SshHost::new("test-server".to_string());
        host.hostname = Some("192.168.1.100".to_string());

        let cloned = host.clone();
        assert_eq!(host, cloned);
        assert_eq!(host.host, cloned.host);
        assert_eq!(host.hostname, cloned.hostname);
    }

    #[test]
    fn test_form_field_readonly() {
        // 测试创建普通字段
        let normal_field = FormField::new("User", "root");
        assert_eq!(normal_field.label, "User");
        assert_eq!(normal_field.value, "root");
        assert!(!normal_field.readonly);

        // 测试创建只读字段
        let readonly_field = FormField::new("Host", "server1").readonly();
        assert_eq!(readonly_field.label, "Host");
        assert_eq!(readonly_field.value, "server1");
        assert!(readonly_field.readonly);

        // 测试链式调用
        let field = FormField::new("Port", "22").readonly();
        assert!(field.readonly);
    }

    #[test]
    fn test_search_persistence_logic() {
        // 测试搜索持久化的核心逻辑
        let mut search_query: Option<String> = None;
        let mut search_input = String::new();

        // 模拟打开搜索弹窗 - 应该为空
        if let Some(ref query) = search_query {
            search_input = query.clone();
        } else {
            search_input.clear();
        }
        assert_eq!(search_input, "");

        // 模拟用户输入 "redis"
        search_input = "redis".to_string();

        // 模拟按 Enter 确认搜索 - 更新持久化的搜索查询
        let query = search_input.trim().to_string();
        search_query = Some(query.clone());

        // 验证搜索查询已保存
        assert_eq!(search_query, Some("redis".to_string()));
        assert_eq!(search_input, "redis");

        // 模拟再次打开搜索弹窗 - 应该恢复之前的搜索文本
        let mut new_search_input = String::new();
        if let Some(ref query) = search_query {
            new_search_input = query.clone();
        } else {
            new_search_input.clear();
        }

        // 验证搜索文本被正确恢复
        assert_eq!(new_search_input, "redis");
    }
}

#[cfg(test)]
mod utils_tests {
    use super::utils::*;

    #[test]
    fn test_validate_port() {
        // 测试有效端口
        assert!(validate_port("22").is_ok());
        assert!(validate_port("80").is_ok());
        assert!(validate_port("443").is_ok());
        assert!(validate_port("8080").is_ok());
        assert!(validate_port("65535").is_ok());

        // 测试无效端口
        assert!(validate_port("0").is_err());
        assert!(validate_port("65536").is_err());
        assert!(validate_port("abc").is_err());
        assert!(validate_port("").is_err());
        assert!(validate_port("-1").is_err());
    }

    #[test]
    fn test_validate_hostname() {
        // 测试有效主机名
        assert!(validate_hostname("example.com").is_ok());
        assert!(validate_hostname("192.168.1.1").is_ok());
        assert!(validate_hostname("localhost").is_ok());
        assert!(validate_hostname("test-server").is_ok());
        assert!(validate_hostname("server.example.org").is_ok());

        // 测试无效主机名
        assert!(validate_hostname("").is_err());
        assert!(validate_hostname(" ").is_err());
        assert!(validate_hostname("invalid..domain").is_err());
    }

    #[test]
    fn test_validate_username() {
        // 测试有效用户名
        assert!(validate_username("user").is_ok());
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("test_user").is_ok());
        assert!(validate_username("user123").is_ok());

        // 测试无效用户名
        assert!(validate_username("").is_err());
        assert!(validate_username(" ").is_err());
        assert!(validate_username("user name").is_err()); // 包含空格
    }
}
