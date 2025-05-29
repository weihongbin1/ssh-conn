//! 国际化模块
//!
//! 支持8种语言的国际化系统，使用YAML配置文件管理翻译内容

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref I18N_INSTANCE: Mutex<I18n> = Mutex::new(I18n::new());
}

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Chinese,
    English,
}

/// YAML翻译文件结构
#[derive(Debug, Deserialize)]
struct TranslationFile {
    ui: Option<HashMap<String, String>>,
    form: Option<HashMap<String, String>>,
    help: Option<HashMap<String, String>>,
    error: Option<HashMap<String, String>>,
    success: Option<HashMap<String, String>>,
    cli: Option<HashMap<String, String>>,
    cli_labels: Option<HashMap<String, String>>,
    validation: Option<HashMap<String, String>>,
    bench: Option<HashMap<String, String>>,
    host_key_confirm: Option<HashMap<String, String>>,
}

impl Language {
    /// 获取语言代码
    pub fn code(&self) -> &'static str {
        match self {
            Language::Chinese => "zh",
            Language::English => "en",
        }
    }

    /// 获取语言名称
    pub fn name(&self) -> &'static str {
        match self {
            Language::Chinese => "中文",
            Language::English => "English",
        }
    }

    /// 从语言代码解析
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "zh" | "zh_cn" | "zh_tw" | "chinese" => Some(Language::Chinese),
            "en" | "en_us" | "en_gb" | "english" => Some(Language::English),
            _ => None,
        }
    }

    /// 获取所有支持的语言
    pub fn all() -> Vec<Language> {
        vec![Language::Chinese, Language::English]
    }

    /// 从环境变量检测语言
    pub fn from_env() -> Self {
        // 检查 SSH_CONN_LANG 环境变量
        if let Ok(ssh_conn_lang) = env::var("SSH_CONN_LANG") {
            if let Some(lang) = Self::from_code(&ssh_conn_lang) {
                return lang;
            }
        }

        // 检查其他环境变量
        let env_vars = ["LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE"];
        for var in &env_vars {
            if let Ok(env_value) = env::var(var) {
                // 提取语言代码部分 (例如: en_US.UTF-8 -> en)
                let lang_part = env_value.split('_').next().unwrap_or("");
                if let Some(lang) = Self::from_code(lang_part) {
                    return lang;
                }
            }
        }

        // 默认中文
        Language::Chinese
    }
}

/// YAML翻译加载器
struct YamlTranslationLoader;

impl YamlTranslationLoader {
    /// 加载指定语言的翻译文件
    fn load_translation_file(&self, lang: &Language) -> Option<TranslationFile> {
        let yaml_content = match lang {
            Language::Chinese => include_str!("../locales/zh.yaml"),
            Language::English => include_str!("../locales/en.yaml"),
        };

        serde_yaml::from_str(yaml_content).ok()
    }

    /// 加载所有翻译到一个HashMap中
    fn load_all_translations(&self, lang: &Language) -> HashMap<String, String> {
        let mut all_translations = HashMap::new();

        if let Some(translation_file) = self.load_translation_file(lang) {
            // 添加UI翻译，前缀为 "ui."
            if let Some(ui_translations) = &translation_file.ui {
                for (key, value) in ui_translations {
                    all_translations.insert(format!("ui.{}", key), value.clone());
                }
            }

            // 添加表单翻译，前缀为 "form."
            if let Some(form_translations) = &translation_file.form {
                for (key, value) in form_translations {
                    all_translations.insert(format!("form.{}", key), value.clone());
                }
            }

            // 添加帮助翻译，前缀为 "help."
            if let Some(help_translations) = &translation_file.help {
                for (key, value) in help_translations {
                    all_translations.insert(format!("help.{}", key), value.clone());
                }
            }

            // 添加错误翻译，前缀为 "error."
            if let Some(error_translations) = &translation_file.error {
                for (key, value) in error_translations {
                    all_translations.insert(format!("error.{}", key), value.clone());
                }
            }

            // 添加成功翻译，前缀为 "success."
            if let Some(success_translations) = &translation_file.success {
                for (key, value) in success_translations {
                    all_translations.insert(format!("success.{}", key), value.clone());
                }
            }

            // 添加CLI翻译，前缀为 "cli."
            if let Some(cli_translations) = &translation_file.cli {
                for (key, value) in cli_translations {
                    all_translations.insert(format!("cli.{}", key), value.clone());
                }
            }

            // 添加CLI标签翻译，前缀为 "cli_labels."
            if let Some(cli_labels_translations) = &translation_file.cli_labels {
                for (key, value) in cli_labels_translations {
                    all_translations.insert(format!("cli_labels.{}", key), value.clone());
                }
            }

            // 添加验证翻译，前缀为 "validation."
            if let Some(validation_translations) = &translation_file.validation {
                for (key, value) in validation_translations {
                    all_translations.insert(format!("validation.{}", key), value.clone());
                }
            }

            // 添加性能测试翻译，前缀为 "bench."
            if let Some(bench_translations) = &translation_file.bench {
                for (key, value) in bench_translations {
                    all_translations.insert(format!("bench.{}", key), value.clone());
                }
            }

            // 添加主机密钥确认翻译，前缀为 "host_key_confirm."
            if let Some(host_key_confirm_translations) = &translation_file.host_key_confirm {
                for (key, value) in host_key_confirm_translations {
                    all_translations.insert(format!("host_key_confirm.{}", key), value.clone());
                }
            }

            // 添加兼容性键（不带前缀）- 常用的UI键
            if let Some(ui_translations) = &translation_file.ui {
                if let Some(value) = ui_translations.get("title") {
                    all_translations.insert("title".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("server_list") {
                    all_translations.insert("server_list".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("search_placeholder") {
                    all_translations.insert("search_placeholder".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("help_text") {
                    all_translations.insert("help_text".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("search_prompt") {
                    all_translations.insert("search_prompt".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("search_input_label") {
                    all_translations.insert("search_input_label".to_string(), value.clone());
                }
                if let Some(value) = ui_translations.get("delete_confirm_title") {
                    all_translations.insert("delete_confirm_title".to_string(), value.clone());
                }
            }

            // 成功消息
            if let Some(success_translations) = &translation_file.success {
                if let Some(value) = success_translations.get("add_server") {
                    all_translations.insert("success_add_server".to_string(), value.clone());
                }
                if let Some(value) = success_translations.get("update_server") {
                    all_translations.insert("success_update_server".to_string(), value.clone());
                }
                if let Some(value) = success_translations.get("delete_server") {
                    all_translations.insert("success_delete_server".to_string(), value.clone());
                }
            }

            // 错误消息
            if let Some(error_translations) = &translation_file.error {
                if let Some(value) = error_translations.get("io_error") {
                    all_translations.insert("error".to_string(), value.clone());
                }
            }

            // 现在直接从YAML的根级别读取兼容性键
            // 这些键在YAML文件中已经定义了
            let yaml_content = match lang {
                Language::Chinese => include_str!("../locales/zh.yaml"),
                Language::English => include_str!("../locales/en.yaml"),
            };

            if let Ok(raw_yaml) = serde_yaml::from_str::<serde_yaml::Value>(yaml_content) {
                if let Some(mapping) = raw_yaml.as_mapping() {
                    for (key, value) in mapping {
                        if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                            // 只添加不是结构体的键
                            if ![
                                "language",
                                "ui",
                                "form",
                                "help",
                                "error",
                                "success",
                                "cli",
                                "cli_labels",
                                "validation",
                                "bench",
                                "host_key_confirm",
                            ]
                            .contains(&key_str)
                            {
                                all_translations.insert(key_str.to_string(), value_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        all_translations
    }
}

/// 国际化管理器
pub struct I18n {
    current_language: Language,
    translation_loader: YamlTranslationLoader,
    cache: HashMap<Language, HashMap<String, String>>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    /// 创建新的国际化管理器
    pub fn new() -> Self {
        let current_language = Language::from_env();
        Self {
            current_language,
            translation_loader: YamlTranslationLoader,
            cache: HashMap::new(),
        }
    }

    /// 设置当前语言
    pub fn set_language(&mut self, language: Language) {
        self.current_language = language;
    }

    /// 获取当前语言
    pub fn current_language(&self) -> Language {
        self.current_language
    }

    /// 获取翻译文本
    pub fn get_text(&mut self, key: &str) -> String {
        // 先检查缓存
        if !self.cache.contains_key(&self.current_language) {
            let translations = self
                .translation_loader
                .load_all_translations(&self.current_language);
            self.cache.insert(self.current_language, translations);
        }

        if let Some(translations) = self.cache.get(&self.current_language) {
            if let Some(text) = translations.get(key) {
                return text.clone();
            }
        }

        // 回退到英文
        if self.current_language != Language::English {
            if !self.cache.contains_key(&Language::English) {
                let translations = self
                    .translation_loader
                    .load_all_translations(&Language::English);
                self.cache.insert(Language::English, translations);
            }

            if let Some(translations) = self.cache.get(&Language::English) {
                if let Some(text) = translations.get(key) {
                    return text.clone();
                }
            }
        }

        // 最终回退到中文
        if self.current_language != Language::Chinese {
            if !self.cache.contains_key(&Language::Chinese) {
                let translations = self
                    .translation_loader
                    .load_all_translations(&Language::Chinese);
                self.cache.insert(Language::Chinese, translations);
            }

            if let Some(translations) = self.cache.get(&Language::Chinese) {
                if let Some(text) = translations.get(key) {
                    return text.clone();
                }
            }
        }

        // 如果都找不到，返回键本身
        key.to_string()
    }

    /// 检查翻译完整度
    pub fn check_translation_completeness(&mut self, language: &Language) -> f64 {
        // 加载英文作为基准
        if !self.cache.contains_key(&Language::English) {
            let translations = self
                .translation_loader
                .load_all_translations(&Language::English);
            self.cache.insert(Language::English, translations);
        }

        // 加载目标语言
        if !self.cache.contains_key(language) {
            let translations = self.translation_loader.load_all_translations(language);
            self.cache.insert(*language, translations);
        }

        let base_translations = self.cache.get(&Language::English).unwrap();
        let target_translations = self.cache.get(language).unwrap();

        let total_keys = base_translations.len();
        let translated_keys = base_translations
            .keys()
            .filter(|key| target_translations.contains_key(*key))
            .count();

        if total_keys == 0 {
            1.0
        } else {
            translated_keys as f64 / total_keys as f64
        }
    }

    /// 列出缺失的翻译
    pub fn list_missing_translations(&mut self, language: &Language) -> Vec<String> {
        // 加载英文作为基准
        if !self.cache.contains_key(&Language::English) {
            let translations = self
                .translation_loader
                .load_all_translations(&Language::English);
            self.cache.insert(Language::English, translations);
        }

        // 加载目标语言
        if !self.cache.contains_key(language) {
            let translations = self.translation_loader.load_all_translations(language);
            self.cache.insert(*language, translations);
        }

        let base_translations = self.cache.get(&Language::English).unwrap();
        let target_translations = self.cache.get(language).unwrap();

        base_translations
            .keys()
            .filter(|key| !target_translations.contains_key(*key))
            .cloned()
            .collect()
    }
}

/// 全局翻译函数
pub fn t(key: &str) -> String {
    I18N_INSTANCE.lock().unwrap().get_text(key)
}

/// 获取当前语言
pub fn current_language() -> Language {
    I18N_INSTANCE.lock().unwrap().current_language()
}

/// 设置当前语言
pub fn set_language(language: Language) {
    I18N_INSTANCE.lock().unwrap().set_language(language);
}

/// 获取所有支持的语言
pub fn supported_languages() -> Vec<Language> {
    Language::all()
}

/// 检查指定语言是否受支持
pub fn is_language_supported(lang: &Language) -> bool {
    Language::all().contains(lang)
}

/// 检查翻译完整度
pub fn check_translation_completeness(language: &Language) -> f64 {
    I18N_INSTANCE
        .lock()
        .unwrap()
        .check_translation_completeness(language)
}

/// 列出缺失的翻译
pub fn list_missing_translations(language: &Language) -> Vec<String> {
    I18N_INSTANCE
        .lock()
        .unwrap()
        .list_missing_translations(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("zh"), Some(Language::Chinese));
        assert_eq!(Language::from_code("zh_CN"), Some(Language::Chinese));
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_language_properties() {
        assert_eq!(Language::Chinese.code(), "zh");
        assert_eq!(Language::English.code(), "en");

        assert_eq!(Language::Chinese.name(), "中文");
        assert_eq!(Language::English.name(), "English");
    }

    #[test]
    fn test_language_detection() {
        // 使用环境变量检测语言的测试
        // 注意：这个测试依赖于实际的环境变量
        let lang = Language::from_env();
        assert!(Language::all().contains(&lang));
    }

    #[test]
    fn test_i18n_get_text() {
        let mut i18n = I18n::new();
        i18n.set_language(Language::English);

        let text = i18n.get_text("ui.title");
        assert!(!text.is_empty());
    }

    #[test]
    fn test_fallback_translation() {
        let mut i18n = I18n::new();
        i18n.set_language(Language::English);

        // 测试回退机制：如果找不到某个键，返回键本身
        let text = i18n.get_text("non_existent_key");
        assert_eq!(text, "non_existent_key");
    }

    #[test]
    fn test_convenience_function() {
        // 测试全局函数
        let text = t("ui.title");
        assert!(!text.is_empty());

        let lang = current_language();
        assert!(supported_languages().contains(&lang));
    }

    #[test]
    fn test_supported_languages() {
        let languages = supported_languages();
        assert_eq!(languages.len(), 2);
        assert!(languages.contains(&Language::Chinese));
        assert!(languages.contains(&Language::English));
    }

    #[test]
    fn test_translation_completeness() {
        let completeness = check_translation_completeness(&Language::English);
        assert!((0.0..=1.0).contains(&completeness));
    }
}
