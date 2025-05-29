# SSH Connection Manager Makefile

# 项目变量
PROJECT_NAME := ssh-conn
DEBUG_TARGET := target/debug/$(PROJECT_NAME)
RELEASE_TARGET := target/release/$(PROJECT_NAME)
INSTALL_PATH := ~/.local/bin

# 多平台构建变量
DIST_DIR := dist
VERSION := $(shell grep version Cargo.toml | head -n1 | cut -d'"' -f2)
TARGETS := x86_64-unknown-linux-gnu \
           x86_64-pc-windows-gnu \
           x86_64-apple-darwin \
           aarch64-apple-darwin \
           aarch64-unknown-linux-gnu

# 使用 cross 进行交叉编译的目标（需要额外的 C 编译器）
CROSS_TARGETS := x86_64-unknown-linux-gnu \
                 x86_64-pc-windows-gnu \
                 aarch64-unknown-linux-gnu

# 本地可直接编译的目标
NATIVE_TARGETS := x86_64-apple-darwin \
                  aarch64-apple-darwin

# 为每个目标平台定义二进制文件扩展名
define get_binary_name
$(if $(findstring windows,$(1)),$(PROJECT_NAME).exe,$(PROJECT_NAME))
endef

# 为每个目标平台定义压缩包名称
define get_archive_name
$(PROJECT_NAME)-$(VERSION)-$(1)$(if $(findstring windows,$(1)),.zip,.tar.gz)
endef

# 默认目标
.DEFAULT_GOAL := help

# 声明伪目标
.PHONY: all build release test fmt check lint run run-release install uninstall clean doc ci pre-release help watch \
        build-cross build-all-targets build-native dist dist-native clean-dist setup-cross check-targets package \
        package-all size-all test-cross check-docker package

# 显示帮助信息
help:
	@echo "SSH连接管理工具 Makefile"
	@echo ""
	@echo "构建目标:"
	@echo "  build        - 构建开发版本"
	@echo "  release      - 构建发布版本"
	@echo "  build-native - 构建本地平台（macOS）所有架构"
	@echo "  build-cross  - 为所有目标平台交叉编译"
	@echo "  build-all-targets - 构建所有目标平台的发布版本"
	@echo "  clean        - 清理构建文件"
	@echo ""
	@echo "多平台打包:"
	@echo "  setup-cross  - 设置交叉编译环境"
	@echo "  check-targets - 检查已安装的目标平台"
	@echo "  check-docker - 检查 Docker 环境（cross 需要）"
	@echo "  dist         - 创建所有平台的发布包（需要 Docker）"
	@echo "  dist-native  - 只创建本地平台（macOS）的发布包"
	@echo "  package      - 为当前平台创建发布包"
	@echo "  package-all  - 为所有平台创建发布包"
	@echo "  clean-dist   - 清理发布包目录"
	@echo ""
	@echo "运行目标:"
	@echo "  run          - 运行开发版本"
	@echo "  run-release  - 运行发布版本"
	@echo "  watch        - 监视文件变化并重新构建运行"
	@echo ""
	@echo "测试目标:"
	@echo "  test         - 运行所有测试"
	@echo "  test-unit    - 只运行单元测试"
	@echo "  test-watch   - 监视测试"
	@echo ""
	@echo "代码质量:"
	@echo "  fmt          - 格式化代码"
	@echo "  check        - 检查代码"
	@echo "  lint         - 运行 clippy 检查"
	@echo "  audit        - 安全审计"
	@echo ""
	@echo "文档和部署:"
	@echo "  doc          - 生成并打开文档"
	@echo "  install      - 安装到系统"
	@echo "  uninstall    - 从系统卸载"
	@echo ""
	@echo "CI/CD:"
	@echo "  ci           - 快速CI检查流程"
	@echo "  ci-full      - 完整CI检查流程（包含安全审计）"
	@echo "  pre-release  - 发布前检查"
	@echo "  pre-release-full - 完整发布前检查（包含安全审计）"
	@echo ""
	@echo "其他:"
	@echo "  deps         - 更新依赖"
	@echo "  size         - 显示二进制文件大小"
	@echo "  size-all     - 显示所有平台二进制文件大小"
	@echo "  test-cross   - 快速测试交叉编译环境"
	@echo "  check-docker - 检查 Docker 环境（cross 需要）"
	@echo "  help         - 显示此帮助信息"

# 默认构建所有
all: build

# 构建开发版本
build:
	@echo "🔨 构建开发版本..."
	cargo build

# 构建发布版本
release:
	@echo "🚀 构建发布版本..."
	cargo build --release

# 运行开发版本
run: build
	@echo "▶️  运行开发版本..."
	cargo run

# 运行发布版本
run-release: release
	@echo "🏃 运行发布版本..."
	cargo run --release

# 监视文件变化并重新构建运行
watch:
	@echo "👀 监视文件变化..."
	@command -v cargo-watch >/dev/null 2>&1 || { echo "请先安装 cargo-watch: cargo install cargo-watch"; exit 1; }
	cargo watch -x run

# 运行所有测试
test:
	@echo "🧪 运行测试..."
	cargo test

# 只运行单元测试
test-unit:
	@echo "🔬 运行单元测试..."
	cargo test --lib

# 监视测试
test-watch:
	@echo "👀 监视测试..."
	@command -v cargo-watch >/dev/null 2>&1 || { echo "请先安装 cargo-watch: cargo install cargo-watch"; exit 1; }
	cargo watch -x test

# 格式化代码
fmt:
	@echo "✨ 格式化代码..."
	cargo fmt --all

# 检查代码
check:
	@echo "🔍 检查代码..."
	cargo check --all-targets

# 运行 clippy 检查
lint:
	@echo "📎 运行 clippy 检查..."
	cargo clippy --all-targets --all-features -- -D warnings

# 安全审计
audit:
	@echo "🔒 安全审计..."
	@command -v cargo-audit >/dev/null 2>&1 || { echo "请先安装 cargo-audit: cargo install cargo-audit"; exit 1; }
	cargo audit

# 生成并打开文档
doc:
	@echo "📚 生成文档..."
	cargo doc --open --no-deps

# 清理构建文件
clean:
	@echo "🧹 清理构建文件..."
	cargo clean
	@if [ -d "$(DIST_DIR)" ]; then \
		echo "清理发布包目录..."; \
		rm -rf $(DIST_DIR); \
	fi
	@echo "✅ 清理完成!"

# 安装到系统
install: release
	@echo "📦 安装到系统..."
	@mkdir -p $(INSTALL_PATH)
	cp $(RELEASE_TARGET) $(INSTALL_PATH)/$(PROJECT_NAME)
	@echo "✅ 已安装到 $(INSTALL_PATH)/$(PROJECT_NAME)"

# 从系统卸载
uninstall:
	@echo "🗑️  从系统卸载..."
	rm -f $(INSTALL_PATH)/$(PROJECT_NAME)
	@echo "✅ 已卸载"

# 更新依赖
deps:
	@echo "📦 更新依赖..."
	cargo update

# 显示二进制文件大小
size: release
	@echo "📏 二进制文件大小:"
	@ls -lh $(RELEASE_TARGET) | awk '{print $$5 "  " $$9}'
	@echo ""
	@echo "详细信息:"
	@file $(RELEASE_TARGET)

# 完整的CI检查流程
ci: fmt check lint test
	@echo "✅ CI检查完成!"

# 完整的CI检查流程（包含安全审计）
ci-full: fmt check lint test audit
	@echo "✅ 完整CI检查完成!"

# 发布前检查
pre-release: ci release size
	@echo "🎉 发布前检查完成!"
	@echo "准备就绪，可以发布!"

# 发布前检查（包含安全审计）
pre-release-full: ci-full release size
	@echo "🎉 完整发布前检查完成!"
	@echo "准备就绪，可以发布!"

# ========== 多平台构建目标 ==========

# 设置交叉编译环境
setup-cross:
	@echo "🔧 设置交叉编译环境..."
	@echo "安装 cross 工具..."
	@if ! command -v cross >/dev/null 2>&1; then \
		echo "安装 cross..."; \
		cargo install cross --git https://github.com/cross-rs/cross; \
	else \
		echo "cross 已安装"; \
	fi
	@echo "添加 Rust 目标平台..."
	@for target in $(TARGETS); do \
		echo "添加目标: $$target"; \
		rustup target add $$target || true; \
	done
	@echo "✅ 交叉编译环境设置完成!"

# 检查已安装的目标平台
check-targets:
	@echo "📋 检查已安装的目标平台:"
	@rustup target list --installed | grep -E "(linux|windows|darwin)" || echo "未找到交叉编译目标"

# 检查 Docker 环境
check-docker:
	@echo "🐳 检查 Docker 环境..."
	@if command -v docker >/dev/null 2>&1; then \
		if docker info >/dev/null 2>&1; then \
			echo "✅ Docker 已安装并运行"; \
		else \
			echo "⚠️  Docker 已安装但未运行，请启动 Docker"; \
			echo "💡 提示: 启动 Docker Desktop 或运行 'sudo systemctl start docker'"; \
		fi; \
	else \
		echo "❌ 未找到 Docker"; \
		echo "💡 提示: cross 工具需要 Docker 来运行交叉编译环境"; \
		echo "   请访问 https://docs.docker.com/get-docker/ 安装 Docker"; \
	fi

# 构建本地平台（macOS）所有架构
build-native:
	@echo "🍎 构建本地平台（macOS）所有架构..."
	@for target in $(NATIVE_TARGETS); do \
		echo "构建目标: $$target"; \
		if cargo build --release --target $$target; then \
			echo "✅ $$target 构建成功"; \
		else \
			echo "❌ $$target 构建失败"; \
		fi; \
	done
	@echo "✅ 本地平台构建完成!"

# 为指定目标构建
build-target-%:
	@echo "🔨 为目标 $* 构建..."
	cargo build --release --target $*

# 交叉编译所有目标
build-cross: setup-cross
	@echo "🌍 开始交叉编译所有目标..."
	@echo "构建本地目标..."
	@for target in $(NATIVE_TARGETS); do \
		echo "构建目标: $$target"; \
		cargo build --release --target $$target || echo "⚠️  目标 $$target 构建失败"; \
	done
	@echo "使用 cross 构建跨平台目标..."
	@for target in $(CROSS_TARGETS); do \
		echo "构建目标: $$target"; \
		cross build --release --target $$target || echo "⚠️  目标 $$target 构建失败"; \
	done
	@echo "✅ 交叉编译完成!"

# 构建所有目标平台的发布版本
build-all-targets: setup-cross
	@echo "🚀 构建所有目标平台的发布版本..."
	@failed_targets=""; \
	for target in $(NATIVE_TARGETS); do \
		echo "构建本地目标: $$target"; \
		if cargo build --release --target $$target; then \
			echo "✅ $$target 构建成功"; \
		else \
			echo "❌ $$target 构建失败"; \
			failed_targets="$$failed_targets $$target"; \
		fi; \
	done; \
	for target in $(CROSS_TARGETS); do \
		echo "使用 cross 构建目标: $$target"; \
		if cross build --release --target $$target; then \
			echo "✅ $$target 构建成功"; \
		else \
			echo "❌ $$target 构建失败"; \
			failed_targets="$$failed_targets $$target"; \
		fi; \
	done; \
	if [ -n "$$failed_targets" ]; then \
		echo "⚠️  以下目标构建失败:$$failed_targets"; \
	else \
		echo "🎉 所有目标构建成功!"; \
	fi

# 为所有平台创建发布包
package-all: build-all-targets
	@echo "📦 为所有平台创建发布包..."
	@mkdir -p $(DIST_DIR)
	@for target in $(TARGETS); do \
		if echo "$$target" | grep -q windows; then \
			binary_name="$(PROJECT_NAME).exe"; \
		else \
			binary_name="$(PROJECT_NAME)"; \
		fi; \
		if echo "$$target" | grep -q windows; then \
			archive_name="$(PROJECT_NAME)-$(VERSION)-$$target.zip"; \
		else \
			archive_name="$(PROJECT_NAME)-$(VERSION)-$$target.tar.gz"; \
		fi; \
		binary_path="target/$$target/release/$$binary_name"; \
		if [ -f "$$binary_path" ]; then \
			echo "打包 $$target..."; \
			if echo "$$target" | grep -q windows; then \
				echo "创建 ZIP 包: $$archive_name"; \
				if command -v zip >/dev/null 2>&1; then \
					zip -j "$(DIST_DIR)/$$archive_name" "$$binary_path" README.md; \
				else \
					echo "⚠️  zip 命令未找到，跳过 $$target"; \
					continue; \
				fi; \
			else \
				echo "创建 TAR.GZ 包: $$archive_name"; \
				tar -czf "$(DIST_DIR)/$$archive_name" -C "target/$$target/release" "$$binary_name" -C ../../../ README.md; \
			fi; \
			echo "✅ $$archive_name 创建完成"; \
		else \
			echo "⚠️  未找到 $$target 的二进制文件: $$binary_path"; \
		fi; \
	done
	@echo "🎉 所有发布包创建完成!"
	@echo "📋 发布包列表:"
	@ls -lah $(DIST_DIR)/

# 创建完整的发布包（包含所有平台）
dist: check-docker package-all
	@echo "🎉 完整发布包创建完成!"
	@echo "📁 发布包位置: $(DIST_DIR)/"
	@echo "📦 包含以下文件:"
	@ls -1 $(DIST_DIR)/

# 只构建和打包本地平台
dist-native: build-native
	@echo "📦 为本地平台创建发布包..."
	@mkdir -p $(DIST_DIR)
	@for target in $(NATIVE_TARGETS); do \
		binary_name=$$(echo "$(call get_binary_name,$$target)"); \
		archive_name=$$(echo "$(call get_archive_name,$$target)"); \
		binary_path="target/$$target/release/$$binary_name"; \
		if [ -f "$$binary_path" ]; then \
			echo "打包 $$target..."; \
			echo "创建 TAR.GZ 包: $$archive_name"; \
			tar -czf "$(DIST_DIR)/$$archive_name" -C "target/$$target/release" "$$binary_name" -C ../../../ README.md; \
			echo "✅ $$archive_name 创建完成"; \
		else \
			echo "⚠️  未找到 $$target 的二进制文件: $$binary_path"; \
		fi; \
	done
	@echo "🎉 本地平台发布包创建完成!"
	@echo "📋 发布包列表:"
	@ls -lah $(DIST_DIR)/

# 清理发布包目录
clean-dist:
	@echo "🧹 清理发布包目录..."
	rm -rf $(DIST_DIR)
	@echo "✅ 发布包目录已清理!"

# 显示所有平台的二进制文件大小
size-all: build-all-targets
	@echo "📏 所有平台二进制文件大小:"
	@for target in $(TARGETS); do \
		binary_name=$$(echo "$(call get_binary_name,$$target)"); \
		binary_path="target/$$target/release/$$binary_name"; \
		if [ -f "$$binary_path" ]; then \
			size=$$(ls -lah "$$binary_path" | awk '{print $$5}'); \
			printf "%-25s %s\n" "$$target:" "$$size"; \
		else \
			printf "%-25s %s\n" "$$target:" "未构建"; \
		fi; \
	done

# 快速测试交叉编译环境
test-cross: setup-cross
	@echo "🧪 测试交叉编译环境..."
	@test_target="x86_64-unknown-linux-gnu"; \
	echo "测试 cross 构建目标: $$test_target"; \
	if cross build --target $$test_target; then \
		echo "✅ 交叉编译环境正常工作!"; \
	else \
		echo "❌ 交叉编译环境可能存在问题"; \
		echo "💡 提示: 确保 Docker 已安装并运行"; \
		exit 1; \
	fi

# 打包发布版本到多平台
package: release
	@echo "📦 打包发布版本到多平台..."
	@mkdir -p $(DIST_DIR)
	@for target in $(TARGETS); do \
		binary_name="$(call get_binary_name,$$target)"; \
		archive_name="$(call get_archive_name,$$target)"; \
		echo "  - 为 $$target 构建并打包..."; \
		cross build --release --target $$target; \
		cp target/$$target/release/$$binary_name $(DIST_DIR)/$$binary_name; \
		tar -czvf $(DIST_DIR)/$$archive_name -C $(DIST_DIR) $$binary_name; \
		rm $(DIST_DIR)/$$binary_name; \
	done
	@echo "✅ 多平台打包完成!"