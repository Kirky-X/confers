#!/bin/bash
# 自动修复代码审查中发现的问题

set -e

echo "🔧 开始自动修复代码审查问题..."
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 1. 修复未使用的导入
echo -e "${YELLOW}1. 修复未使用的导入...${NC}"
cargo fix --lib --allow-dirty
cargo fix --bin --allow-dirty
echo -e "${GREEN}✓ 未使用的导入已修复${NC}"
echo ""

# 2. 创建修复 main.rs 的补丁
echo -e "${YELLOW}2. 修复 main.rs 条件编译问题...${NC}"

# 备份原文件
cp src/main.rs src/main.rs.backup

# 创建修复后的 main.rs
cat > src/main.rs << 'EOF'
// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

#[cfg(feature = "cli")]
use confers::commands::{
    completions::CompletionsCommand,
    diff::{DiffCommand, DiffFormat, DiffOptions},
    encrypt::EncryptCommand,
    generate::GenerateCommand,
    key::KeyCommand,
    validate::{ValidateCommand, ValidateLevel},
    wizard::ConfigWizard,
};

use confers::ConfigError;

#[cfg(feature = "cli")]
use std::str::FromStr;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "confers")]
#[command(about = "Configuration management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// 生成配置模板
    Generate {
        /// 输出文件路径
        #[arg(short, long)]
        output: Option<String>,

        /// 模板级别 (minimal, full)
        #[arg(short, long, default_value = "full")]
        level: String,
    },
    /// 验证配置文件
    Validate {
        /// 配置文件路径
        #[arg(short, long)]
        config: String,

        /// 输出级别 (minimal, full, documentation)
        #[arg(short, long, default_value = "full")]
        level: String,
    },
    /// 对比两个配置文件
    Diff {
        /// 第一个文件
        file1: String,
        /// 第二个文件
        file2: String,

        /// 输出样式 (unified, context, normal, side-by-side, strict)
        #[arg(short, long)]
        style: Option<String>,
    },
    /// 生成 Shell 补全脚本
    Completions {
        /// 要生成补全的 Shell 类型
        shell: String,
    },
    /// 加密一个值
    Encrypt {
        /// 要加密的值
        value: String,

        /// 加密密钥（Base64 编码，32 字节）。如未提供，则使用 CONFERS_ENCRYPTION_KEY 环境变量。
        #[arg(short, long)]
        key: Option<String>,
    },
    /// 交互式配置向导
    Wizard {
        /// 跳过交互式提示，使用默认值
        #[arg(long)]
        non_interactive: bool,
    },
    /// 密钥管理操作
    #[command(subcommand)]
    Key(#[command(subcommand)] confers::commands::key::KeySubcommand),
}

#[cfg(feature = "cli")]
fn main() -> Result<(), ConfigError> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generate { output, level } => {
            GenerateCommand::execute_placeholder(output.as_ref(), level)?;
        }
        Commands::Validate { config, level } => {
            let validate_level = ValidateLevel::parse(level);
            ValidateCommand::execute_generic(config, validate_level)?;
        }
        Commands::Diff {
            file1,
            file2,
            style,
        } => {
            let diff_format = DiffFormat::from_str(style.as_deref().unwrap_or("unified"))
                .map_err(ConfigError::ParseError)?;
            let options = DiffOptions {
                format: diff_format,
                ..DiffOptions::default()
            };
            DiffCommand::execute(file1, file2, options)?;
        }
        Commands::Completions { shell } => {
            CompletionsCommand::execute::<Cli>(shell)?;
        }
        Commands::Encrypt { value, key } => {
            EncryptCommand::execute(value, key.as_ref())?;
        }
        Commands::Wizard { non_interactive } => {
            let wizard = ConfigWizard::new();
            if *non_interactive {
                // 在非交互模式下使用默认值
                let values = &["", "", "", "", "", "", ""];
                let config = wizard.run_with_values(values)?;
                config.save()?;
            } else {
                let config = wizard.run()?;
                config.save()?;
            }
        }
        Commands::Key(subcommand) => {
            KeyCommand::execute(subcommand, None, None)?;
        }
    }

    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() -> Result<(), ConfigError> {
    eprintln!("❌ Error: CLI feature is not enabled.");
    eprintln!("");
    eprintln!("The confers CLI tool requires the 'cli' feature to be enabled.");
    eprintln!("");
    eprintln!("To build the CLI tool, use one of the following commands:");
    eprintln!("  cargo build --features cli");
    eprintln!("  cargo build --features dev");
    eprintln!("  cargo build --features full");
    eprintln!("");
    eprintln!("For library-only usage, you can use:");
    eprintln!("  cargo build --features minimal");
    eprintln!("  cargo build --features recommended");
    eprintln!("");
    eprintln!("See the documentation for more information on feature presets.");
    std::process::exit(1);
}
EOF

echo -e "${GREEN}✓ main.rs 条件编译已修复${NC}"
echo ""

# 3. 修复 watcher 模块
echo -e "${YELLOW}3. 修复 watcher 模块未使用的导入...${NC}"

# 备份原文件
cp src/watcher/mod.rs src/watcher/mod.rs.backup

# 修复 watcher 模块的导入
# 移除未使用的导入，将条件导入移到条件编译块内
cat > /tmp/watcher_fix.txt << 'EOF'
--- a/src/watcher/mod.rs
+++ b/src/watcher/mod.rs
@@ -3,8 +3,6 @@
 // See LICENSE file in the project root for full license information.
 
-use crate::core::loader::is_editor_temp_file;
-use crate::error::ConfigError;
-
 #[cfg(feature = "watch")]
 use notify::{RecursiveMode, Watcher};
 #[cfg(feature = "watch")]
 use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
 use std::path::PathBuf;
-use std::sync::mpsc::{channel, Receiver};
 use std::time::{Duration, Instant};
 
 #[cfg(feature = "remote")]
 use crate::utils::ssrf::validate_remote_url;
 #[cfg(feature = "remote")]
 use reqwest;
 #[cfg(feature = "remote")]
 use tokio::time::interval;
 
 #[cfg(feature = "remote")]
 use std::fs;
 
+#[cfg(feature = "watch")]
+use std::sync::mpsc::{channel, Receiver};
+
+#[cfg(feature = "watch")]
+use crate::core::loader::is_editor_temp_file;
+
+#[cfg(feature = "watch")]
+use crate::error::ConfigError;
+
EOF

# 应用修复（需要手动应用，因为 patch 可能不完美）
echo -e "${YELLOW}  请手动应用 watcher 模块的修复${NC}"
echo -e "${YELLOW}  或运行: patch -p1 < /tmp/watcher_fix.txt${NC}"
echo ""

# 4. 验证修复
echo -e "${YELLOW}4. 验证修复...${NC}"

# 测试 minimal 特性
echo "  测试 minimal 特性..."
if cargo build --no-default-features --features minimal --lib 2>&1 | grep -q "Finished"; then
    echo -e "${GREEN}  ✓ minimal 特性编译成功${NC}"
else
    echo -e "${RED}  ✗ minimal 特性编译失败${NC}"
fi

# 测试 recommended 特性
echo "  测试 recommended 特性..."
if cargo build --no-default-features --features recommended --lib 2>&1 | grep -q "Finished"; then
    echo -e "${GREEN}  ✓ recommended 特性编译成功${NC}"
else
    echo -e "${RED}  ✗ recommended 特性编译失败${NC}"
fi

# 测试 CLI 特性
echo "  测试 CLI 特性..."
if cargo build --no-default-features --features cli 2>&1 | grep -q "Finished"; then
    echo -e "${GREEN}  ✓ CLI 特性编译成功${NC}"
else
    echo -e "${RED}  ✗ CLI 特性编译失败${NC}"
fi

echo ""
echo -e "${GREEN}✅ 自动修复完成！${NC}"
echo ""
echo "📝 剩余需要手动修复的问题："
echo "  1. 完成 encryption 功能集成 (medium)"
echo "  2. 减少代码重复 (medium)"
echo "  3. 更新文档 (medium)"
echo "  4. 手动修复 watcher 模块导入 (high)"
echo ""
echo "📄 详细修复说明请查看: CODE_REVIEW_REPORT.md"
echo ""
echo "🔍 查看备份文件："
echo "  - src/main.rs.backup"
echo "  - src/watcher/mod.rs.backup"