//! 终端用户界面模块

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use std::io;

use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::ConfigManager;
use crate::i18n::t;
use crate::models::{ConnectionStatus, FormField, SshHost};

/// 连接测试结果类型别名
type PendingConnectionTests = Arc<Mutex<Vec<(usize, Option<ConnectionStatus>)>>>;

/// 搜索状态
#[derive(Default)]
struct SearchState {
    query: Option<String>,
    show_popup: bool,
    input: String,
}

/// 删除确认状态
#[derive(Default)]
struct DeleteConfirmState {
    show: bool,
    host: Option<String>,
    input: String,
}

/// 表单状态
#[derive(Default)]
struct FormState {
    show_add: bool,
    show_edit: bool,
    fields: Vec<FormField>,
    focus_index: usize,
    editing_field: bool,
    edit_host_original: Option<SshHost>,
    error_field_index: Option<usize>,
}

/// 错误模态框状态
#[derive(Default)]
struct ErrorModalState {
    show: bool,
    message: String,
}

/// 主机密钥确认状态
#[derive(Default)]
struct HostKeyConfirmState {
    show: bool,
    host: Option<String>,
    selection: usize, // 0: Yes, 1: No
}

/// UI状态管理器
#[derive(Default)]
struct UiState {
    search: SearchState,
    delete_confirm: DeleteConfirmState,
    form: FormState,
    error_modal: ErrorModalState,
    host_key_confirm: HostKeyConfirmState,
}

/// 终端UI管理器
pub struct UiManager {
    config_manager: ConfigManager,
    state: UiState,
    /// 正在进行的连接测试结果
    pending_connection_tests: PendingConnectionTests,
}

impl UiManager {
    /// 创建一个新的UI管理器
    pub fn new(config_manager: ConfigManager) -> Self {
        Self {
            config_manager,
            state: UiState::default(),
            pending_connection_tests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 显示错误信息模态框
    fn show_error_message(&mut self, message: &str) -> io::Result<()> {
        self.state.error_modal.message = message.to_string();
        self.state.error_modal.show = true;
        Ok(())
    }

    /// 显示错误信息并标记错误字段
    fn show_error_with_field(&mut self, message: &str, field_index: usize) -> io::Result<()> {
        self.state.error_modal.message = message.to_string();
        self.state.error_modal.show = true;
        self.state.form.error_field_index = Some(field_index);
        Ok(())
    }
    /// 启动TUI界面
    pub fn start_tui(&mut self) -> io::Result<()> {
        // 检查是否有主机配置
        let hosts = self.config_manager.get_hosts()?.clone();
        if hosts.is_empty() {
            println!("{}", t("error.no_servers_found"));
            return Ok(());
        }

        let mut terminal = self.setup_terminal()?;
        let (mut hosts, mut selected, mut table_state) = Self::initialize_state(&hosts);

        // 自动触发全部服务器的连接测试
        self.test_all_connections(&mut hosts);

        self.main_event_loop(&mut terminal, &mut hosts, &mut selected, &mut table_state)?;

        Self::cleanup_terminal()?;
        Ok(())
    }

    /// 设置终端
    fn setup_terminal(&self) -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        Terminal::new(backend)
    }

    /// 初始化状态
    fn initialize_state(
        hosts: &[crate::models::SshHost],
    ) -> (Vec<crate::models::SshHost>, usize, TableState) {
        let selected = 0;
        let mut table_state = TableState::default();
        table_state.select(Some(selected));
        let hosts = hosts.to_vec();
        (hosts, selected, table_state)
    }

    /// 主事件循环
    fn main_event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<crate::models::SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        let mut error_count = 0;
        const MAX_ERRORS: u32 = 5;

        loop {
            // 检查并更新连接测试结果
            self.update_connection_test_results(hosts);

            // 渲染界面，如果渲染失败则尝试恢复
            if let Err(e) = self.render_ui(terminal, hosts, table_state) {
                error_count += 1;
                if error_count >= MAX_ERRORS {
                    // 错误次数过多，执行紧急恢复
                    self.emergency_terminal_recovery()?;
                    return Err(e);
                }

                // 尝试恢复终端并继续
                self.emergency_terminal_recovery()?;
                // 额外重新初始化事件系统
                let _ = self.reinitialize_event_system();
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            // 处理事件，如果返回true则退出循环
            if self.process_events(terminal, hosts, selected, table_state)? {
                break;
            }

            // 重置错误计数
            error_count = 0;

            // 确保界面及时刷新，防止SSH连接后界面冻结
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }
    /// 渲染UI
    fn render_ui(
        &self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &[crate::models::SshHost],
        table_state: &mut TableState,
    ) -> io::Result<()> {
        terminal.draw(|f| {
            let size = f.area();

            // 渲染搜索输入框
            let y_offset = self.render_search_popup(f, size);

            // 渲染主表格
            self.render_main_table(f, size, y_offset, hosts, table_state);

            // 渲染各种弹窗
            self.render_delete_confirm_popup(f, size);
            self.render_form_popup(f, size);
            self.render_error_modal(f, size);
            self.render_host_key_confirm(f, size);
        })?;
        Ok(())
    }

    /// 处理事件
    fn process_events(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<crate::models::SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        // 使用较短的超时时间，确保界面响应及时
        if !event::poll(std::time::Duration::from_millis(100))? {
            return Ok(false);
        }

        if let Event::Key(key) = event::read()? {
            // 处理错误模态框
            if self.state.error_modal.show {
                self.handle_error_modal();
                return Ok(false);
            }

            // 处理各种弹窗状态
            if self.state.search.show_popup {
                if self.handle_search_event(key.code, hosts, selected, table_state)? {
                    return Ok(false);
                }
            } else if self.state.host_key_confirm.show {
                if self.handle_host_key_event(key.code, terminal, hosts, selected, table_state)? {
                    return Ok(false);
                }
            } else if self.state.delete_confirm.show {
                if self.handle_delete_confirm_event(key.code, hosts, selected, table_state)? {
                    return Ok(false);
                }
            } else if self.state.form.show_add || self.state.form.show_edit {
                if self.handle_form_event(key.code, hosts, selected, table_state)? {
                    return Ok(false);
                }
            } else {
                // 处理主界面事件
                return self.handle_main_event(key.code, terminal, hosts, selected, table_state);
            }
        }

        Ok(false)
    }

    /// 处理错误模态框
    fn handle_error_modal(&mut self) {
        self.state.error_modal.show = false;
        self.state.error_modal.message.clear();
        self.state.form.error_field_index = None;
    }

    /// 清理终端
    fn cleanup_terminal() -> io::Result<()> {
        // 执行完整的终端清理，确保程序退出时终端状态正常
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        // 额外的终端恢复，确保完全清理
        use std::process::Command;
        let _ = Command::new("stty").args(["sane"]).status();
        let _ = Command::new("tput").args(["cnorm"]).status(); // 恢复光标

        Ok(())
    }

    /// 渲染搜索弹窗
    fn render_search_popup(&self, f: &mut ratatui::Frame, size: Rect) -> u16 {
        if !self.state.search.show_popup {
            return 0;
        }

        let search_block = Block::default()
            .borders(Borders::ALL)
            .title(t("ui.search_prompt"));
        let search_area = Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: 3,
        };
        let lines = [format!(
            "{}: {}█",
            t("ui.search_input_label"),
            self.state.search.input
        )];
        let para = Paragraph::new(lines.join("\n")).alignment(Alignment::Left);

        f.render_widget(search_block, search_area);
        f.render_widget(
            para,
            Rect {
                x: 2,
                y: 1,
                width: size.width - 4,
                height: 2,
            },
        );
        3
    }

    /// 渲染删除确认弹窗
    fn render_delete_confirm_popup(&self, f: &mut ratatui::Frame, size: Rect) {
        if !self.state.delete_confirm.show {
            return;
        }

        let popup_area = self.centered_rect(50, 20, size);
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        f.render_widget(Clear, popup_area);

        let delete_block = Block::default()
            .title(format!("⚠️  {}", t("ui.delete_confirm_title")))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Red).fg(Color::White));
        f.render_widget(delete_block, popup_area);

        let unknown = t("unknown");
        let host_name = self
            .state
            .delete_confirm
            .host
            .as_deref()
            .unwrap_or(&unknown);
        let confirm_text = t("ui.delete_confirm_message").replace("{}", host_name);
        let input_text =
            t("ui.delete_confirm_input").replace("{}", &self.state.delete_confirm.input);
        let warning_text = t("ui.delete_confirm_warning");
        let esc_text = t("ui.delete_confirm_esc");

        let delete_text = [
            "",
            &confirm_text,
            "",
            &warning_text,
            "",
            &input_text,
            "",
            &esc_text,
            "",
        ];
        let delete_paragraph = Paragraph::new(delete_text.join("\n"))
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::White));
        f.render_widget(delete_paragraph, inner_area);
    }

    /// 渲染表单弹窗
    fn render_form_popup(&self, f: &mut ratatui::Frame, size: Rect) {
        if !self.state.form.show_add && !self.state.form.show_edit {
            return;
        }

        let popup_area = self.centered_rect(70, 80, size);
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        f.render_widget(Clear, popup_area);

        let title = if self.state.form.show_add {
            t("ui.add_server_form_title")
        } else {
            t("ui.edit_server_form_title")
        };

        let form_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(form_block, popup_area);

        if !self.state.form.fields.is_empty() {
            let form_text = self.build_form_text();
            let form_paragraph = Paragraph::new(form_text.join("\n"))
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::White))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(form_paragraph, inner_area);
        }
    }

    /// 渲染主表格
    fn render_main_table(
        &self,
        f: &mut ratatui::Frame,
        size: Rect,
        y_offset: u16,
        hosts: &[SshHost],
        table_state: &mut TableState,
    ) {
        let table_area = Rect {
            x: 0,
            y: y_offset,
            width: size.width,
            height: size.height - y_offset,
        };

        let header = Row::new(vec![
            Cell::from("Host"),
            Cell::from("HostName"),
            Cell::from("User"),
            Cell::from("Port"),
            Cell::from("Status"),
            Cell::from("ProxyCommand"),
            Cell::from("IdentityFile"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = hosts
            .iter()
            .map(|h| {
                Row::new(vec![
                    Cell::from(h.host.clone()),
                    Cell::from(h.hostname.clone().unwrap_or_default()),
                    Cell::from(h.user.clone().unwrap_or_default()),
                    Cell::from(h.port.clone().unwrap_or_default()),
                    Cell::from(h.connection_status.display_string()),
                    Cell::from(h.proxy_command.clone().unwrap_or_default()),
                    Cell::from(h.identity_file.clone().unwrap_or_default()),
                ])
            })
            .collect();

        let title = if let Some(query) = &self.state.search.query {
            format!(
                "{} ({}: {}) ({})",
                t("ui.server_list"),
                t("ui.search_result"),
                query,
                t("help.help_navigation")
            )
        } else {
            format!("{} ({})", t("ui.server_list"), t("help.help_navigation"))
        };

        let table = Table::new(
            rows,
            &[
                Constraint::Min(15),    // Host 列 - 最小15字符
                Constraint::Min(15),    // HostName 列 - 最小15字符
                Constraint::Length(8),  // User 列
                Constraint::Length(6),  // Port 列
                Constraint::Length(12), // Status 列
                Constraint::Min(20),    // ProxyCommand 列 - 最小20字符
                Constraint::Min(20),    // IdentityFile 列 - 最小20字符
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("▍ ");
        f.render_stateful_widget(table, table_area, table_state);
    }

    /// 构建表单文本
    fn build_form_text(&self) -> Vec<String> {
        let mut form_text = Vec::new();

        for (i, field) in self.state.form.fields.iter().enumerate() {
            let is_error_field = self.state.form.error_field_index == Some(i);
            let is_readonly = self.state.form.show_edit && i == 0;

            let line = self.format_form_field(i, field, is_error_field, is_readonly);
            form_text.push(line);
        }

        form_text.push(String::new());
        if self.state.form.editing_field {
            form_text.push(t("ui.form_complete_enter"));
            if self.state.form.show_edit {
                form_text.push(format!("🔒 {}", t("ui.host_readonly_hint")));
            }
        } else {
            form_text.push(t("ui.form_shortcuts"));
            if self.state.form.show_edit {
                form_text.push(format!("🔒 {}", t("ui.host_readonly_hint")));
            }
        }

        form_text
    }

    /// 格式化表单字段
    fn format_form_field(
        &self,
        index: usize,
        field: &FormField,
        is_error: bool,
        is_readonly: bool,
    ) -> String {
        let is_focused = index == self.state.form.focus_index;
        let is_editing = self.state.form.editing_field && is_focused;

        match (is_focused, is_editing, is_readonly, is_error) {
            (true, true, false, false) => format!("▶ {}: {}█", field.label, field.value),
            (true, true, false, true) => format!("▶ ❌ {}: {}█", field.label, field.value),
            (true, true, true, false) => format!("▶ 🔒 {}: {}█", field.label, field.value),
            (true, true, true, true) => format!("▶ 🔒 ❌ {}: {}█", field.label, field.value),
            (true, false, true, false) => format!("▶ 🔒 {}: {}", field.label, field.value),
            (true, false, true, true) => format!("▶ 🔒 ❌ {}: {}", field.label, field.value),
            (true, false, false, false) => format!("▶ {}: {}", field.label, field.value),
            (true, false, false, true) => format!("▶ ❌ {}: {}", field.label, field.value),
            (false, _, true, false) => format!("  🔒 {}: {}", field.label, field.value),
            (false, _, true, true) => format!("  🔒 ❌ {}: {}", field.label, field.value),
            (false, _, false, false) => format!("  {}: {}", field.label, field.value),
            (false, _, false, true) => format!("  ❌ {}: {}", field.label, field.value),
        }
    }

    /// 渲染错误模态框
    fn render_error_modal(&self, f: &mut ratatui::Frame, size: Rect) {
        if !self.state.error_modal.show {
            return;
        }

        let popup_area = self.centered_rect(60, 30, size);
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        f.render_widget(Clear, popup_area);

        let error_block = Block::default()
            .title(format!("❌ {}", t("error.prefix")))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Red).fg(Color::White));
        f.render_widget(error_block, popup_area);

        let press_any_key_text = t("press_any_key");
        let error_text = [
            "",
            &self.state.error_modal.message,
            "",
            &press_any_key_text,
            "",
        ];
        let error_paragraph = Paragraph::new(error_text.join("\n"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));
        f.render_widget(error_paragraph, inner_area);
    }

    /// 渲染主机密钥确认对话框
    fn render_host_key_confirm(&self, f: &mut ratatui::Frame, size: Rect) {
        if !self.state.host_key_confirm.show {
            return;
        }

        let popup_area = self.centered_rect(60, 40, size);
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        f.render_widget(Clear, popup_area);

        let host_key_block = Block::default()
            .title(t("host_key_verification_title"))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Yellow).fg(Color::Black));
        f.render_widget(host_key_block, popup_area);

        let unknown = t("unknown");
        let host_name = self
            .state
            .host_key_confirm
            .host
            .as_deref()
            .unwrap_or(&unknown);
        let mut content_lines = vec![
            "".to_string(),
            format!(
                "{}",
                t("host_key_confirm.warning_title").replace("{}", host_name)
            ),
            "".to_string(),
            t("host_key_confirm.possible_reasons"),
            t("host_key_confirm.reason_1"),
            t("host_key_confirm.reason_2"),
            "".to_string(),
            t("host_key_confirm.question"),
            "".to_string(),
        ];

        let yes_text = if self.state.host_key_confirm.selection == 0 {
            format!(
                "▶ [ {} ]   [ {} ]",
                t("host_key_confirm.yes_option"),
                t("host_key_confirm.no_option")
            )
        } else {
            format!(
                "  [ {} ] ▶ [ {} ]",
                t("host_key_confirm.yes_option"),
                t("host_key_confirm.no_option")
            )
        };
        content_lines.push(format!("    {}", yes_text));
        content_lines.push("".to_string());
        content_lines.push(format!("    {}", t("host_key_confirm.shortcuts")));

        let host_key_paragraph = Paragraph::new(content_lines.join("\n"))
            .alignment(Alignment::Left)
            .style(Style::default().fg(Color::Black));
        f.render_widget(host_key_paragraph, inner_area);
    }

    /// 计算居中弹窗的位置
    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    /// 保存表单数据
    fn save_form_data(
        &mut self,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        // 验证必填字段
        if self.state.form.fields.len() < 2 {
            self.show_error_message(&t("error.error_required_fields"))?;
            return Ok(false);
        }

        // 验证Host字段
        if self.state.form.fields[0].value.is_empty() {
            self.show_error_with_field(&t("error.error_required_fields"), 0)?;
            // 设置焦点到Host字段并进入编辑模式
            self.state.form.focus_index = 0;
            self.state.form.editing_field = true;
            return Ok(false);
        }

        // 验证HostName字段
        if self.state.form.fields[1].value.is_empty() {
            self.show_error_with_field(&t("error.error_required_fields"), 1)?;
            // 设置焦点到HostName字段并进入编辑模式
            self.state.form.focus_index = 1;
            self.state.form.editing_field = true;
            return Ok(false);
        }

        // 验证端口号
        let port = if self.state.form.fields[3].value.is_empty() {
            None
        } else {
            match self.state.form.fields[3].value.parse::<u16>() {
                Ok(p) => {
                    if p == 0 {
                        self.show_error_with_field(&t("error.error_port_range"), 3)?;
                        // 设置焦点到端口字段并进入编辑模式
                        self.state.form.focus_index = 3;
                        self.state.form.editing_field = true;
                        return Ok(false);
                    }
                    Some(p)
                }
                Err(_) => {
                    self.show_error_with_field(&t("error.error_port_format"), 3)?;
                    // 设置焦点到端口字段并进入编辑模式
                    self.state.form.focus_index = 3;
                    self.state.form.editing_field = true;
                    return Ok(false);
                }
            }
        };

        // 保存数据
        let result = if self.state.form.show_add {
            // 添加主机
            self.config_manager.add_host(
                &self.state.form.fields[0].value,
                &self.state.form.fields[1].value,
                if self.state.form.fields[2].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[2].value)
                },
                port,
                if self.state.form.fields[4].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[4].value)
                },
                if self.state.form.fields[5].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[5].value)
                },
                if self.state.form.fields[6].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[6].value)
                },
            )
        } else {
            // 编辑主机
            self.config_manager.edit_host(
                &self.state.form.fields[0].value,
                if self.state.form.fields[1].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[1].value)
                },
                if self.state.form.fields[2].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[2].value)
                },
                port,
                if self.state.form.fields[4].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[4].value)
                },
                if self.state.form.fields[5].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[5].value)
                },
                if self.state.form.fields[6].value.is_empty() {
                    None
                } else {
                    Some(&self.state.form.fields[6].value)
                },
            )
        };

        match result {
            Ok(_) => {
                // 保存成功，重新加载主机列表
                self.config_manager.clear_cache();
                *hosts = self.config_manager.get_hosts()?.clone();

                if self.state.form.show_add {
                    *selected = 0;
                } else if *selected >= hosts.len() && !hosts.is_empty() {
                    *selected = hosts.len() - 1;
                }

                if !hosts.is_empty() {
                    table_state.select(Some(*selected));
                } else {
                    table_state.select(None);
                }

                Ok(true)
            }
            Err(e) => {
                self.show_error_message(&e.to_string())?;
                Ok(false)
            }
        }
    }

    /// 处理搜索弹窗事件
    fn handle_search_event(
        &mut self,
        key: KeyCode,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        match key {
            KeyCode::Enter => {
                let query = self.state.search.input.trim().to_string();
                if query.is_empty() {
                    self.state.search.query = None;
                    *hosts = self.config_manager.get_hosts()?.clone();
                } else {
                    self.state.search.query = Some(query.clone());
                    *hosts = self.config_manager.search_hosts(&query)?;
                }
                *selected = 0;
                if !hosts.is_empty() {
                    table_state.select(Some(*selected));
                } else {
                    table_state.select(None);
                }
                self.state.search.show_popup = false;
                self.state.search.input.clear();
                Ok(true)
            }
            KeyCode::Esc => {
                self.state.search.show_popup = false;
                self.state.search.input.clear();
                Ok(true)
            }
            KeyCode::Char(c) => {
                self.state.search.input.push(c);
                self.update_search_results(hosts, selected, table_state)?;
                Ok(true)
            }
            KeyCode::Backspace => {
                self.state.search.input.pop();
                self.update_search_results(hosts, selected, table_state)?;
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    /// 更新搜索结果
    fn update_search_results(
        &mut self,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        let query = self.state.search.input.trim();
        if query.is_empty() {
            self.state.search.query = None;
            *hosts = self.config_manager.get_hosts()?.clone();
        } else {
            self.state.search.query = Some(query.to_string());
            *hosts = self.config_manager.search_hosts(query)?;
        }
        *selected = 0;
        if !hosts.is_empty() {
            table_state.select(Some(*selected));
        } else {
            table_state.select(None);
        }
        Ok(())
    }

    /// 处理删除确认事件
    fn handle_delete_confirm_event(
        &mut self,
        key: KeyCode,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        match key {
            KeyCode::Enter => {
                if self.state.delete_confirm.input.trim().to_lowercase() == "yes" {
                    if let Some(host_to_delete) = &self.state.delete_confirm.host {
                        let _ = self.config_manager.delete_host(host_to_delete);
                        self.reset_delete_confirm();
                        self.reload_hosts(hosts, selected, table_state)?;
                    }
                }
                Ok(true)
            }
            KeyCode::Esc => {
                self.reset_delete_confirm();
                Ok(true)
            }
            KeyCode::Char(c) => {
                self.state.delete_confirm.input.push(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.state.delete_confirm.input.pop();
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    /// 重置删除确认状态
    fn reset_delete_confirm(&mut self) {
        self.state.delete_confirm.show = false;
        self.state.delete_confirm.host = None;
        self.state.delete_confirm.input.clear();
    }

    /// 重新加载主机列表
    fn reload_hosts(
        &mut self,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        self.config_manager.clear_cache();
        *hosts = self.config_manager.get_hosts()?.clone();
        if *selected >= hosts.len() && !hosts.is_empty() {
            *selected = hosts.len() - 1;
        }
        if !hosts.is_empty() {
            table_state.select(Some(*selected));
        } else {
            table_state.select(None);
        }
        Ok(())
    }

    /// 处理表单事件
    fn handle_form_event(
        &mut self,
        key: KeyCode,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        match key {
            KeyCode::Esc => {
                if self.state.form.editing_field {
                    self.state.form.editing_field = false;
                } else {
                    self.reset_form();
                }
                Ok(true)
            }
            KeyCode::Char('q') if !self.state.form.editing_field => {
                self.reset_form();
                Ok(true)
            }
            KeyCode::Char('q') if self.state.form.editing_field => {
                if self.state.form.focus_index < self.state.form.fields.len() {
                    self.state.form.fields[self.state.form.focus_index]
                        .value
                        .push('q');
                }
                Ok(true)
            }
            KeyCode::Tab | KeyCode::Down if !self.state.form.editing_field => {
                self.move_form_focus_down();
                Ok(true)
            }
            KeyCode::Up if !self.state.form.editing_field => {
                self.move_form_focus_up();
                Ok(true)
            }
            KeyCode::Enter => {
                self.handle_form_enter();
                Ok(true)
            }
            KeyCode::Char('s') if !self.state.form.editing_field => {
                if self.save_form_data(hosts, selected, table_state)? {
                    self.reset_form();
                }
                Ok(true)
            }
            KeyCode::Char('s') if self.state.form.editing_field => {
                if self.state.form.focus_index < self.state.form.fields.len() {
                    self.state.form.fields[self.state.form.focus_index]
                        .value
                        .push('s');
                }
                Ok(true)
            }
            KeyCode::Char(c) if self.state.form.editing_field => {
                self.handle_form_input(c);
                Ok(true)
            }
            KeyCode::Backspace if self.state.form.editing_field => {
                self.handle_form_backspace();
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    /// 重置表单状态
    fn reset_form(&mut self) {
        self.state.form.show_add = false;
        self.state.form.show_edit = false;
        self.state.form.fields.clear();
        self.state.form.focus_index = 0;
        self.state.form.editing_field = false;
        self.state.form.edit_host_original = None;
        self.state.form.error_field_index = None;
    }

    /// 移动表单焦点到下一个字段
    fn move_form_focus_down(&mut self) {
        if !self.state.form.fields.is_empty() {
            let mut next_index = (self.state.form.focus_index + 1) % self.state.form.fields.len();
            if self.state.form.show_edit && next_index == 0 && self.state.form.fields.len() > 1 {
                next_index = (next_index + 1) % self.state.form.fields.len();
            }
            self.state.form.focus_index = next_index;
        }
    }

    /// 移动表单焦点到上一个字段
    fn move_form_focus_up(&mut self) {
        if !self.state.form.fields.is_empty() {
            let mut prev_index = if self.state.form.focus_index == 0 {
                self.state.form.fields.len() - 1
            } else {
                self.state.form.focus_index - 1
            };
            if self.state.form.show_edit && prev_index == 0 && self.state.form.fields.len() > 1 {
                prev_index = if prev_index == 0 {
                    self.state.form.fields.len() - 1
                } else {
                    prev_index - 1
                };
            }
            self.state.form.focus_index = prev_index;
        }
    }

    /// 处理表单Enter键
    fn handle_form_enter(&mut self) {
        if self.state.form.editing_field {
            self.state.form.editing_field = false;
            if self.state.form.focus_index + 1 < self.state.form.fields.len() {
                self.state.form.focus_index += 1;
                self.state.form.editing_field = true;
            }
        } else if self.state.form.show_edit && self.state.form.focus_index == 0 {
            if self.state.form.focus_index + 1 < self.state.form.fields.len() {
                self.state.form.focus_index += 1;
                self.state.form.editing_field = true;
            }
        } else {
            self.state.form.editing_field = true;
            if self.state.form.error_field_index == Some(self.state.form.focus_index) {
                self.state.form.error_field_index = None;
            }
        }
    }

    /// 处理表单字符输入
    fn handle_form_input(&mut self, c: char) {
        if self.state.form.focus_index < self.state.form.fields.len()
            && !(self.state.form.show_edit && self.state.form.focus_index == 0)
        {
            self.state.form.fields[self.state.form.focus_index]
                .value
                .push(c);
        }
    }

    /// 处理表单退格键
    fn handle_form_backspace(&mut self) {
        if self.state.form.focus_index < self.state.form.fields.len()
            && !(self.state.form.show_edit && self.state.form.focus_index == 0)
        {
            self.state.form.fields[self.state.form.focus_index]
                .value
                .pop();
        }
    }

    /// 处理主机密钥确认事件
    fn handle_host_key_event(
        &mut self,
        key: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        match key {
            KeyCode::Enter => {
                if let Some(host) = self.state.host_key_confirm.host.clone() {
                    if self.state.host_key_confirm.selection == 0 {
                        self.handle_host_key_accept(&host, terminal, hosts, selected, table_state)?;
                    }
                }
                self.reset_host_key_confirm();
                Ok(true)
            }
            KeyCode::Esc => {
                self.reset_host_key_confirm();
                Ok(true)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.state.host_key_confirm.selection = 0;
                Ok(true)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.state.host_key_confirm.selection = 1;
                Ok(true)
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(host) = self.state.host_key_confirm.host.clone() {
                    self.handle_host_key_accept(&host, terminal, hosts, selected, table_state)?;
                }
                self.reset_host_key_confirm();
                Ok(true)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.reset_host_key_confirm();
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    /// 重置主机密钥确认状态
    fn reset_host_key_confirm(&mut self) {
        self.state.host_key_confirm.show = false;
        self.state.host_key_confirm.host = None;
        self.state.host_key_confirm.selection = 0;
    }

    /// 处理主机密钥接受
    fn handle_host_key_accept(
        &mut self,
        host: &str,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        // 1. 退出TUI模式，恢复正常终端
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        // 2. 使用TUI专用的主机密钥处理方法
        let result = self
            .config_manager
            .handle_host_key_verification_failed_for_tui(host);

        // 3. 等待系统稳定，防止终端状态混乱
        std::thread::sleep(std::time::Duration::from_millis(300));

        // 4. 重新初始化终端环境 - 增强版
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        // 5. 强制清理终端，确保主机密钥处理后状态完全正常
        execute!(
            io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;

        // 6. 清除任何可能残留的按键事件
        while event::poll(std::time::Duration::from_millis(1))? {
            let _ = event::read()?;
        }

        // 6. 重新创建终端后端，确保完全重置
        let backend = CrosstermBackend::new(io::stdout());
        *terminal = Terminal::new(backend)?;

        // 7. 强制清屏，确保界面干净
        terminal.clear()?;

        // 8. 刷新服务器列表数据和UI状态
        self.refresh_after_connection(hosts, selected, table_state)?;

        // 9. 额外确保事件系统工作正常
        self.reinitialize_event_system()?;

        // 10. 强制重新渲染整个界面，确保主机密钥处理后界面正确显示
        self.force_render_ui(terminal, hosts, table_state)?;

        // 10. 如果连接有错误，显示错误信息
        if let Err(e) = result {
            self.show_error_message(
                &t("host_key_processing_failed").replace("{}", &e.to_string()),
            )?;
        }

        Ok(())
    }

    /// 退出TUI并连接
    ///
    /// 此方法处理SSH连接的完整流程：
    /// 1. 退出TUI模式
    /// 2. 执行SSH连接
    /// 3. 重新进入TUI模式
    /// 4. 刷新界面数据并强制重新渲染
    fn exit_and_connect(
        &mut self,
        host: &str,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        // 1. 退出TUI模式，恢复正常终端
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        // 2. 执行SSH连接
        let connection_result = self.config_manager.connect_host_for_tui(host);

        // 3. 等待系统稳定，防止终端状态混乱
        std::thread::sleep(std::time::Duration::from_millis(200));

        // 4. 重新初始化终端环境 - 增强版
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        // 5. 强制清理终端，确保SSH连接后状态完全正常
        execute!(
            io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;

        // 6. 清除任何可能残留的按键事件，防止SSH会话的按键影响UI
        while event::poll(std::time::Duration::from_millis(1))? {
            let _ = event::read()?;
        }

        // 6. 重新创建终端后端，确保完全重置
        let backend = CrosstermBackend::new(io::stdout());
        *terminal = Terminal::new(backend)?;

        // 7. 强制清屏，确保界面干净
        terminal.clear()?;

        // 8. 刷新服务器列表数据和UI状态
        self.refresh_after_connection(hosts, selected, table_state)?;

        // 9. 额外确保事件系统工作正常
        self.reinitialize_event_system()?;

        // 10. 强制重新渲染整个界面，确保SSH连接后界面正确显示
        self.force_render_ui(terminal, hosts, table_state)?;

        // 10. 如果连接有错误，显示错误信息
        if let Err(e) = connection_result {
            self.show_error_message(&format!("{}: {}", t("error.connection_failed"), e))?;
        }
        Ok(())
    }

    /// 连接后刷新界面
    fn refresh_after_connection(
        &mut self,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        // 1. 强化终端状态恢复 - 确保终端设置完全正确
        use std::process::Command;

        // 执行多重终端修复，确保彻底恢复正常状态
        let restore_commands = [
            vec!["stty", "sane"],                             // 重置到安全状态
            vec!["stty", "echo", "icanon", "onlcr", "icrnl"], // 恢复标准设置
            vec!["tput", "sgr0"],                             // 重置所有终端属性
            vec!["tput", "cnorm"],                            // 恢复光标显示
            vec!["tput", "clear"],                            // 清屏
        ];

        for cmd_args in restore_commands.iter() {
            let _ = Command::new(cmd_args[0]).args(&cmd_args[1..]).status();
        }

        // 2. 等待终端状态稳定
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. 强制重新初始化终端模式，确保按键捕获正常
        disable_raw_mode()?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        enable_raw_mode()?;

        // 4. 清除任何可能残留的事件
        while event::poll(std::time::Duration::from_millis(1))? {
            let _ = event::read()?;
        }

        // 5. 重新初始化所有UI状态
        self.reset_all_ui_state();

        // 6. 强制重新初始化事件系统，确保按键响应正常
        self.reinitialize_event_system()?;

        // 6. 重新加载服务器列表数据
        if let Some(query) = &self.state.search.query {
            // 如果当前有搜索查询，重新执行搜索
            if let Ok(search_results) = self.config_manager.search_hosts(query) {
                *hosts = search_results;
            }
        } else {
            // 否则加载所有主机
            if let Ok(all_hosts) = self.config_manager.get_hosts() {
                *hosts = all_hosts.clone();
            }
        }

        // 确保选中索引有效
        if *selected >= hosts.len() && !hosts.is_empty() {
            *selected = hosts.len() - 1;
        }

        // 更新表格状态
        if !hosts.is_empty() {
            table_state.select(Some(*selected));
        } else {
            table_state.select(None);
            *selected = 0;
        }

        Ok(())
    }

    /// 强制重新渲染UI界面
    ///
    /// 专门用于SSH连接后的界面重新渲染，确保：
    /// 1. 清除SSH会话可能留下的终端状态
    /// 2. 重新绘制完整的TUI界面
    /// 3. 恢复正确的表格选中状态
    fn force_render_ui(
        &self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &[crate::models::SshHost],
        table_state: &mut TableState,
    ) -> io::Result<()> {
        // 强制重新渲染界面，确保SSH连接后界面正确显示
        terminal.draw(|f| {
            let size = f.area();

            // 渲染搜索输入框
            let y_offset = self.render_search_popup(f, size);

            // 渲染主表格
            self.render_main_table(f, size, y_offset, hosts, table_state);

            // 渲染各种弹窗
            self.render_delete_confirm_popup(f, size);
            self.render_form_popup(f, size);
            self.render_error_modal(f, size);
            self.render_host_key_confirm(f, size);
        })?;
        Ok(())
    }

    /// 重置所有UI状态
    ///
    /// 在SSH连接后重置所有可能被影响的UI状态，确保界面完全可用
    fn reset_all_ui_state(&mut self) {
        // 重置所有弹窗状态
        self.state.search.show_popup = false;
        self.state.search.input.clear();

        self.state.delete_confirm.show = false;
        self.state.delete_confirm.host = None;
        self.state.delete_confirm.input.clear();

        self.state.form.show_add = false;
        self.state.form.show_edit = false;
        self.state.form.fields.clear();
        self.state.form.focus_index = 0;
        self.state.form.editing_field = false;
        self.state.form.edit_host_original = None;
        self.state.form.error_field_index = None;

        self.state.error_modal.show = false;
        self.state.error_modal.message.clear();

        self.state.host_key_confirm.show = false;
        self.state.host_key_confirm.host = None;
        self.state.host_key_confirm.selection = 0;
    }

    /// 检查并更新连接测试结果
    fn update_connection_test_results(&mut self, hosts: &mut [SshHost]) {
        if let Ok(mut pending_tests) = self.pending_connection_tests.lock() {
            let mut completed_indices = Vec::new();

            for (i, (host_index, status_opt)) in pending_tests.iter().enumerate() {
                if let Some(status) = status_opt {
                    if *host_index < hosts.len() {
                        hosts[*host_index].connection_status = status.clone();
                        completed_indices.push(i);
                    }
                }
            }

            // 移除已完成的测试（从后往前移除以避免索引问题）
            for &i in completed_indices.iter().rev() {
                pending_tests.remove(i);
            }
        }
    }

    /// 处理主界面事件
    fn handle_main_event(
        &mut self,
        key: KeyCode,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<bool> {
        match key {
            KeyCode::Char('q') => Ok(true), // 退出
            KeyCode::Down => {
                if !hosts.is_empty() && *selected < hosts.len() - 1 {
                    *selected += 1;
                    table_state.select(Some(*selected));
                }
                Ok(false)
            }
            KeyCode::Up => {
                if !hosts.is_empty() && *selected > 0 {
                    *selected -= 1;
                    table_state.select(Some(*selected));
                }
                Ok(false)
            }
            KeyCode::Enter => {
                if !hosts.is_empty() {
                    let host = hosts[*selected].host.clone();
                    self.handle_connect_request(&host, terminal, hosts, selected, table_state)?;
                }
                Ok(false)
            }
            KeyCode::Char('a') => {
                self.show_add_form();
                Ok(false)
            }
            KeyCode::Char('e') => {
                if !hosts.is_empty() {
                    self.show_edit_form(&hosts[*selected]);
                }
                Ok(false)
            }
            KeyCode::Char('d') => {
                if !hosts.is_empty() {
                    self.show_delete_confirm(&hosts[*selected].host);
                }
                Ok(false)
            }
            KeyCode::Char('s') | KeyCode::Char('/') => {
                self.show_search_popup();
                Ok(false)
            }
            KeyCode::Char('t') => {
                if !hosts.is_empty() {
                    self.start_connection_test(hosts, *selected);
                }
                Ok(false)
            }
            KeyCode::Char('T') => {
                if !hosts.is_empty() {
                    self.test_all_connections(hosts);
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// 处理连接请求
    fn handle_connect_request(
        &mut self,
        host: &str,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        hosts: &mut Vec<SshHost>,
        selected: &mut usize,
        table_state: &mut TableState,
    ) -> io::Result<()> {
        let (success, host_key_error, error_message) = self.config_manager.try_connect_host(host);

        if host_key_error {
            self.state.host_key_confirm.show = true;
            self.state.host_key_confirm.host = Some(host.to_string());
            self.state.host_key_confirm.selection = 0;
        } else if !success {
            if let Some(err_msg) = error_message {
                self.show_error_message(&format!("{}: {}", t("error.connection_failed"), err_msg))?;
            } else {
                self.show_error_message(&t("error.connection_failed"))?;
            }
        } else {
            // 连接测试成功，进行实际的SSH连接
            self.exit_and_connect(host, terminal, hosts, selected, table_state)?;
        }
        Ok(())
    }

    /// 显示添加表单
    fn show_add_form(&mut self) {
        self.state.form.show_add = true;
        self.state.form.fields = vec![
            FormField::new(t("form.host"), ""),
            FormField::new(t("form.hostname"), ""),
            FormField::new(t("form.user"), ""),
            FormField::new(t("form.port"), ""),
            FormField::new(t("form.proxy_command"), ""),
            FormField::new(t("form.identity_file"), ""),
            FormField::new(t("form.password"), ""),
        ];
        self.state.form.focus_index = 0;
        self.state.form.editing_field = false;
    }

    /// 显示编辑表单
    fn show_edit_form(&mut self, host: &SshHost) {
        self.state.form.show_edit = true;
        self.state.form.edit_host_original = Some(host.clone());
        self.state.form.fields = vec![
            FormField::new(t("form.host"), &host.host),
            FormField::new(
                t("form.hostname"),
                host.hostname.clone().unwrap_or_default(),
            ),
            FormField::new(t("form.user"), host.user.clone().unwrap_or_default()),
            FormField::new(t("form.port"), host.port.clone().unwrap_or_default()),
            FormField::new(
                t("form.proxy_command"),
                host.proxy_command.clone().unwrap_or_default(),
            ),
            FormField::new(
                t("form.identity_file"),
                host.identity_file.clone().unwrap_or_default(),
            ),
            FormField::new(t("form.password"), ""),
        ];
        self.state.form.focus_index = 1; // 编辑模式下，初始焦点设在第二个字段
        self.state.form.editing_field = false;
    }

    /// 显示删除确认
    fn show_delete_confirm(&mut self, host: &str) {
        self.state.delete_confirm.show = true;
        self.state.delete_confirm.host = Some(host.to_string());
        self.state.delete_confirm.input.clear();
    }

    /// 显示搜索弹窗
    fn show_search_popup(&mut self) {
        self.state.search.show_popup = true;
        if let Some(ref query) = self.state.search.query {
            self.state.search.input = query.clone();
        } else {
            self.state.search.input.clear();
        }
    }

    /// 启动连接测试
    fn start_connection_test(&mut self, hosts: &mut [SshHost], selected: usize) {
        if selected >= hosts.len() {
            return;
        }

        // 设置状态为连接中
        hosts[selected].connection_status = ConnectionStatus::Connecting;

        // 克隆必要的数据
        let mut host = hosts[selected].clone();
        let pending_tests = self.pending_connection_tests.clone();

        // 添加到待处理列表
        if let Ok(mut pending) = pending_tests.lock() {
            pending.push((selected, None));
        }

        // 在独立线程中运行连接测试
        thread::spawn(move || {
            // 创建运行时并执行测试
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("Failed to create async runtime: {}", e);
                    let error_status = ConnectionStatus::Failed("Runtime error".to_string());
                    if let Ok(mut pending) = pending_tests.lock() {
                        if let Some(entry) = pending.iter_mut().find(|(idx, _)| *idx == selected) {
                            entry.1 = Some(error_status);
                        }
                    }
                    return;
                }
            };

            // 执行连接测试
            let result_status = rt.block_on(async {
                match host.test_connection().await {
                    Ok(_) => host.connection_status.clone(),
                    Err(_) => host.connection_status.clone(),
                }
            });

            // 更新结果
            if let Ok(mut pending) = pending_tests.lock() {
                if let Some(entry) = pending.iter_mut().find(|(idx, _)| *idx == selected) {
                    entry.1 = Some(result_status);
                }
            }

            log::info!(
                "Connection test completed for {}: {}",
                host.host,
                host.connection_status.detail_string()
            );
        });
    }

    /// 批量测试所有主机连接
    fn test_all_connections(&mut self, hosts: &mut [SshHost]) {
        // 设置所有主机状态为连接中
        for (index, host) in hosts.iter_mut().enumerate() {
            host.connection_status = ConnectionStatus::Connecting;

            // 克隆必要的数据
            let mut host_clone = host.clone();
            let pending_tests = self.pending_connection_tests.clone();

            // 添加到待处理列表
            if let Ok(mut pending) = pending_tests.lock() {
                pending.push((index, None));
            }

            // 在独立线程中运行连接测试
            thread::spawn(move || {
                // 创建运行时并执行测试
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        log::error!("Failed to create async runtime: {}", e);
                        let error_status = ConnectionStatus::Failed("Runtime error".to_string());
                        if let Ok(mut pending) = pending_tests.lock() {
                            if let Some(entry) = pending.iter_mut().find(|(idx, _)| *idx == index) {
                                entry.1 = Some(error_status);
                            }
                        }
                        return;
                    }
                };

                // 执行连接测试
                let result_status = rt.block_on(async {
                    match host_clone.test_connection().await {
                        Ok(_) => host_clone.connection_status.clone(),
                        Err(_) => host_clone.connection_status.clone(),
                    }
                });

                // 更新结果
                if let Ok(mut pending) = pending_tests.lock() {
                    if let Some(entry) = pending.iter_mut().find(|(idx, _)| *idx == index) {
                        entry.1 = Some(result_status);
                    }
                }

                log::debug!(
                    "Connection test completed for {}: {}",
                    host_clone.host,
                    host_clone.connection_status.detail_string()
                );
            });
        }

        log::info!("Started batch connection test for {} hosts", hosts.len());
    }

    /// 强制重新初始化事件系统
    ///
    /// 在SSH连接后确保事件处理系统完全重置，解决按键无响应的问题
    fn reinitialize_event_system(&self) -> io::Result<()> {
        // 1. 刷新stdout，清除任何缓冲数据
        use std::io::Write;
        io::stdout().flush()?;

        // 2. 强制重新初始化crossterm事件队列
        // 清除任何可能残留的事件
        while event::poll(std::time::Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        // 3. 短暂禁用再重新启用raw mode以强制重置
        disable_raw_mode()?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        enable_raw_mode()?;

        Ok(())
    }

    /// 安全终端恢复
    ///
    /// 在发生意外情况时尝试恢复终端到可用状态
    fn emergency_terminal_recovery(&self) -> io::Result<()> {
        use std::process::Command;

        // 尝试多种终端恢复方法
        let recovery_commands = [
            vec!["stty", "sane"],
            vec!["reset"],
            vec!["tput", "cnorm"], // 恢复光标
            vec!["tput", "sgr0"],  // 重置属性
        ];

        for cmd_args in recovery_commands.iter() {
            let _ = Command::new(cmd_args[0]).args(&cmd_args[1..]).output(); // 使用output而不是status，避免输出干扰
        }

        Ok(())
    }
}
