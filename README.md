# SSH Connection Manager (ssh-conn)

<div align="center">

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](Cargo.toml)

**一个现代化的SSH连接管理工具，用Rust编写，支持CLI和TUI双界面**

[功能特性](#功能特性) • [快速开始](#快速开始) • [使用指南](#使用指南) • [配置](#配置) • [贡献](#贡献)

</div>

---

## 🚀 功能特性

### 核心功能
- **�️ 双界面模式**: 命令行界面 (CLI) 和终端用户界面 (TUI)，满足不同使用场景
- **🔐 安全密码管理**: SQLite加密存储，自动登录无需重复输入密码
- **📝 智能配置管理**: 自动解析和修改SSH配置文件，支持所有标准SSH选项
- **🔍 快速搜索**: 支持主机名、用户名、地址等多字段模糊搜索
- **🌐 连接状态监控**: 实时显示服务器连通性和网络延迟

### 高级特性
- **✅ 智能错误处理**: 表单验证自动定位错误字段，视觉化错误提示
- **🛡️ 连接回退机制**: 自动登录失败时智能回退到标准SSH连接
- **📦 配置备份**: 自动备份SSH配置文件，确保数据安全
- **⚡ 批量连接测试**: 支持单个或全部服务器连接状态检测
- **🌍 国际化支持**: 支持中文和英文界面

## 📋 系统要求

### 必需组件
- **Rust**: 1.75+ (推荐使用最新稳定版)
- **操作系统**: Linux / macOS / Windows

### 可选组件 (推荐安装)
为了使用自动密码输入功能，建议安装 `sshpass`:

<details>
<summary>🍎 macOS</summary>

```bash
brew install sshpass
```
</details>

<details>
<summary>🐧 Ubuntu/Debian</summary>

```bash
sudo apt-get install sshpass
```
</details>

<details>
<summary>🎩 CentOS/RHEL/Fedora</summary>

```bash
# CentOS/RHEL
sudo yum install sshpass

# Fedora
sudo dnf install sshpass
```
</details>

## 🚀 快速开始

### 方式一：从源码构建 (推荐)

```bash
# 1. 克隆仓库
git clone https://github.com/weihongbin1/ssh-conn.git
cd ssh-conn

# 2. 构建发布版本
cargo build --release

# 3. 安装到系统 (可选)
cargo install --path .

# 4. 运行程序
./target/release/ssh-conn
```

### 方式二：直接安装

```bash
# 如果已发布到 crates.io
cargo install ssh-conn
```

## 📖 使用指南

### 🎯 快速上手

#### 1. 启动 TUI 界面 (推荐新用户)
```bash
ssh-conn
```

#### 2. 命令行模式快速操作
```bash
# 查看所有服务器
ssh-conn list

# 连接到服务器
ssh-conn connect myserver

# 添加新服务器
ssh-conn add production 192.168.1.100 --user admin --port 22
```

### 🖥️ TUI 模式详解

启动 TUI 界面后，使用以下快捷键操作：

| 快捷键 | 功能 | 说明 |
|--------|------|------|
| `↑` / `↓` | 导航选择 | 在服务器列表中上下移动 |
| `Enter` | 连接服务器 | 连接到当前选中的服务器 |
| `a` | 添加服务器 | 打开添加服务器表单 |
| `e` | 编辑服务器 | 编辑当前选中的服务器配置 |
| `d` | 删除服务器 | 删除当前选中的服务器 |
| `s` | 搜索服务器 | 打开搜索对话框 |
| `t` | 测试连接 | 测试当前选中服务器的连通性 |
| `T` | 批量测试 | 测试所有服务器的连通性 |
| `q` | 退出程序 | 安全退出应用程序 |

#### 连接状态指示器
服务器列表中的状态列实时显示连接状态：

| 图标 | 状态 | 说明 |
|------|------|------|
| ⚪ | 未检测 | 尚未进行连接测试 |
| 🟡 | 连接中... | 正在进行连接测试 |
| 🟢 | 已连接 (15ms) | 连接成功，显示响应时间 |
| 🔴 | 连接失败 | 无法连接到服务器 |

### ⌨️ 命令行模式详解

<details>
<summary>📋 查看服务器列表</summary>

```bash
ssh-conn list
```
显示所有配置的SSH服务器及其详细信息。
</details>

<details>
<summary>🔗 连接到服务器</summary>

```bash
ssh-conn connect <主机名>
```
连接到指定的SSH服务器。如果设置了密码，将自动登录。
</details>

<details>
<summary>➕ 添加新服务器</summary>

```bash
ssh-conn add <主机名> <地址> [选项]

# 选项:
#   -u, --user <用户名>           登录用户名
#   -p, --port <端口>             SSH端口 (默认: 22)
#   --proxy-command <命令>        代理命令
#   --identity-file <文件路径>    私钥文件路径

# 示例:
ssh-conn add webserver 192.168.1.100 --user admin --port 2222
ssh-conn add jumpbox 10.0.0.1 --proxy-command "ProxyJump bastion"
```
</details>

<details>
<summary>✏️ 编辑服务器配置</summary>

```bash
ssh-conn edit <主机名> [选项]

# 只更新指定的选项，其他配置保持不变
ssh-conn edit webserver --hostname 192.168.1.101 --port 22
```
</details>

<details>
<summary>🗑️ 删除服务器</summary>

```bash
ssh-conn delete <主机名>
```
从配置中删除指定的服务器。
</details>

<details>
<summary>🔍 搜索服务器</summary>

```bash
ssh-conn search <关键词>

# 示例:
ssh-conn search prod      # 搜索包含 "prod" 的服务器
ssh-conn search 192.168   # 搜索特定IP段
```
</details>

<details>
<summary>💾 备份配置</summary>

```bash
ssh-conn backup
```
创建当前SSH配置文件的备份。
</details>

## 🔐 自动密码功能

### 工作原理
SSH连接管理工具提供智能的密码管理功能，让连接更加便捷和安全：

1. **🔒 安全存储**: 密码使用 SQLite 数据库加密存储在本地
2. **⚡ 自动登录**: 连接时自动使用存储的密码，无需重复输入  
3. **🛡️ 智能回退**: 自动登录失败时，无缝回退到标准SSH连接模式
4. **🧹 内存保护**: 密码在内存中仅短暂存在，使用后立即清除

### 使用方式
- 在 TUI 界面的添加/编辑表单中设置密码
- 首次连接时会提示设置密码（可选）
- 支持随时更新或删除存储的密码

### 安全特性
- ✅ 本地加密存储，不上传任何数据
- ✅ 支持所有标准SSH安全选项
- ✅ 密码加密存储在 `~/.ssh/ssh_conn_passwords.db`
- ✅ 兼容SSH密钥认证，密码仅作为备选方案

## ⚙️ 配置

### 文件位置
- **SSH配置**: `~/.ssh/config` (标准SSH配置文件)
- **密码数据库**: `~/.ssh/ssh_conn_passwords.db`
- **备份文件**: `~/.ssh/config.backup.YYYYMMDD_HHMMSS`

### 支持的SSH配置选项

| 选项 | 说明 | 示例 |
|------|------|------|
| **Host** | 主机别名 | `webserver` |
| **HostName** | 实际主机地址 | `192.168.1.100` |
| **User** | 登录用户名 | `admin` |
| **Port** | SSH端口 | `22`, `2222` |
| **ProxyCommand** | 代理命令 | `ProxyJump bastion` |
| **IdentityFile** | 私钥文件路径 | `~/.ssh/id_rsa` |
| **ConnectTimeout** | 连接超时时间 | `10` |
| **ServerAliveInterval** | 心跳间隔 | `60` |
| **自定义选项** | 其他SSH选项 | `Compression yes` |

### 配置示例

```ssh-config
Host production
    HostName 192.168.1.100
    User admin
    Port 22
    IdentityFile ~/.ssh/prod_key
    ConnectTimeout 10
    ServerAliveInterval 60

Host staging
    HostName staging.example.com
    User deploy
    Port 2222
    ProxyCommand ssh -W %h:%p bastion
```

## 🏗️ 项目架构

```
ssh-conn/
├── src/
│   ├── main.rs          # 🚀 程序入口点
│   ├── lib.rs           # 📚 库入口和公共接口
│   ├── cli.rs           # 💻 命令行接口实现
│   ├── ui.rs            # 🖥️ TUI界面管理
│   ├── config.rs        # ⚙️ SSH配置文件解析和管理
│   ├── password.rs      # 🔐 密码安全存储和管理
│   ├── network.rs       # 🌐 网络连接测试
│   ├── models.rs        # 📋 数据模型定义
│   ├── error.rs         # ❌ 错误处理和类型定义
│   ├── utils.rs         # 🛠️ 通用工具函数
│   └── i18n.rs          # 🌍 国际化支持
├── locales/
│   ├── zh.yaml          # 🇨🇳 中文翻译
│   └── en.yaml          # 🇺🇸 英文翻译
├── Cargo.toml           # 📦 项目配置和依赖
└── README.md            # 📖 项目文档
```

## 🔧 技术栈

### 核心依赖

| 依赖项 | 版本 | 用途 |
|--------|------|------|
| **clap** | 4.5+ | 命令行参数解析 |
| **ratatui** | 0.30+ | 终端用户界面框架 |
| **crossterm** | 0.29+ | 跨平台终端控制 |
| **rusqlite** | 0.36+ | SQLite数据库操作 |
| **tokio** | 1.0+ | 异步运行时 |
| **serde** | 1.0+ | 序列化/反序列化 |
| **thiserror** | 1.0+ | 错误处理 |
| **dirs** | 6.0+ | 系统目录定位 |
| **chrono** | 0.4+ | 日期时间处理 |

### 开发工具依赖
- **tempfile**: 测试临时文件
- **log** + **env_logger**: 日志系统

## 🧪 开发指南

### 环境准备
```bash
# 安装 Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/weihongbin1/ssh-conn.git
cd ssh-conn
```

### 开发工作流
```bash
# 运行开发版本
cargo run

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查 (推荐)
cargo clippy

# 构建发布版本
cargo build --release
```

### 测试策略
```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test config
cargo test network

# 运行集成测试
cargo test --test integration

# 测试覆盖率 (需要安装 cargo-tarpaulin)
cargo tarpaulin
```

## 🤝 贡献指南

我们欢迎所有形式的贡献！

### 如何参与
1. **🍴 Fork** 项目到你的GitHub
2. **🌿 创建分支** (`git checkout -b feature/amazing-feature`)
3. **💻 编写代码** 并确保测试通过
4. **📝 提交变更** (`git commit -m 'Add amazing feature'`)
5. **🚀 推送分支** (`git push origin feature/amazing-feature`)
6. **📮 创建 Pull Request**

### 贡献类型
- 🐛 **Bug报告**: 发现问题？请创建issue
- 💡 **功能建议**: 有好想法？我们想听听
- 📚 **文档改进**: 让文档更清晰易懂
- 🧪 **测试用例**: 增加测试覆盖率
- 🌍 **翻译**: 支持更多语言

### 开发规范
- 遵循 Rust 官方代码风格 (`cargo fmt`)
- 通过所有 Clippy 检查 (`cargo clippy`)
- 为新功能添加测试用例
- 更新相关文档

## 📄 开源协议

本项目基于 [MIT License](LICENSE) 开源协议。

## 🔒 安全注意事项

### 密码安全
- ✅ 密码使用SQLite数据库本地加密存储
- ✅ 建议为密码数据库设置访问权限 (600)
- ⚠️ 不要在公共仓库中提交包含敏感信息的配置文件

### SSH密钥安全
- ✅ SSH私钥文件应设置正确的权限 (600)
- ✅ 推荐优先使用SSH密钥认证而非密码
- ✅ 定期更换和管理SSH密钥

### 网络安全
- ✅ 使用 `StrictHostKeyChecking` 防止中间人攻击
- ✅ 支持 ProxyJump 等高级SSH选项
- ✅ 自动备份配置文件防止数据丢失

## ❓ 常见问题

<details>
<summary><strong>🔑 如何管理SSH密钥和密码？</strong></summary>

**Q**: 工具如何处理SSH密钥和密码的关系？  
**A**: 
- SSH密钥认证是首选方式，工具完全兼容标准SSH密钥认证
- 密码功能是可选的补充，主要用于没有配置密钥的服务器
- 如果服务器同时配置了密钥和密码，会优先尝试密钥认证
- 密码仅在密钥认证失败或不可用时作为备选方案

</details>

<details>
<summary><strong>🛠️ SSH连接后终端显示异常怎么办？</strong></summary>

**Q**: 连接SSH后回车不换行或显示异常？  
**A**: 工具已实现智能终端修复机制：

**🔧 自动修复 (推荐)**:
- 程序会在SSH连接后自动检测和修复终端状态
- 使用 `-tt` 和 `RequestTTY=force` 强制分配伪终端
- 自动设置正确的换行参数 (onlcr、echo、icanon等)

**🛠️ 手动修复**:
```bash
# 快速修复命令
stty sane

# 完整重置 (严重情况)
reset && stty sane && clear
```

</details>

<details>
<summary><strong>🔍 如何搜索和过滤服务器？</strong></summary>

**Q**: 如何快速找到需要的服务器？  
**A**: 
- **TUI模式**: 按 `s` 键打开搜索对话框
- **CLI模式**: 使用 `ssh-conn search <关键词>`
- **搜索范围**: 支持主机名、地址、用户名等多字段搜索
- **模糊匹配**: 不区分大小写，支持部分匹配

</details>

<details>
<summary><strong>📝 程序会修改我的SSH配置文件吗？</strong></summary>

**Q**: 使用工具是否安全？会不会损坏现有配置？  
**A**: 
- ✅ 程序直接读写标准的 `~/.ssh/config` 文件
- ✅ 每次修改前自动创建备份文件
- ✅ 完全兼容标准SSH配置格式
- ✅ 支持所有SSH配置选项，不会丢失自定义配置
- 💡 建议首次使用前手动备份配置文件

</details>

<details>
<summary><strong>🌍 支持哪些操作系统？</strong></summary>

**Q**: 在哪些系统上可以运行？  
**A**: 
- ✅ **Linux**: 完全支持 (Ubuntu, CentOS, Arch等)
- ✅ **macOS**: 完全支持 (Intel/Apple Silicon)
- ✅ **Windows**: 支持 (需要SSH客户端，推荐WSL2)
- 📋 **依赖**: 需要系统中安装SSH客户端

</details>

<details>
<summary><strong>🚨 连接失败或错误如何处理？</strong></summary>

**Q**: SSH连接失败时如何诊断问题？  
**A**: 工具提供多层次的错误处理：

- **🔍 智能诊断**: 自动识别常见错误（主机密钥、网络超时等）
- **📊 连接测试**: 使用 `t` 键测试单个服务器或 `T` 键批量测试
- **🛡️ 自动回退**: 自动登录失败时回退到标准SSH连接
- **📝 详细日志**: 设置 `RUST_LOG=debug` 获取详细调试信息
- **⚡ 状态显示**: 实时显示连接状态和延迟信息

</details>

<details>
<summary><strong>🎨 如何自定义界面和行为？</strong></summary>

**Q**: 可以自定义程序的界面或行为吗？  
**A**: 
- **🌍 语言**: 自动检测系统语言，支持中文/英文切换
- **⚙️ SSH选项**: 支持所有标准SSH配置选项
- **🎯 快捷键**: TUI界面提供丰富的快捷键操作
- **📋 CLI模式**: 完整的命令行接口，适合脚本自动化

</details>

---

<div align="center">

**🌟 如果这个项目对你有帮助，请考虑给个 Star！**

[![GitHub stars](https://img.shields.io/github/stars/weihongbin1/ssh-conn?style=social)](https://github.com/weihongbin1/ssh-conn/stargazers)

[📖 文档](README.md) • [🐛 报告问题](https://github.com/weihongbin1/ssh-conn/issues) • [💡 功能建议](https://github.com/weihongbin1/ssh-conn/issues) • [🤝 参与贡献](https://github.com/weihongbin1/ssh-conn/pulls)

</div>
