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
    eprintln!();
    eprintln!("The confers CLI tool requires the 'cli' feature to be enabled.");
    eprintln!();
    eprintln!("To build the CLI tool, use one of the following commands:");
    eprintln!("  cargo build --features cli");
    eprintln!("  cargo build --features dev");
    eprintln!("  cargo build --features full");
    eprintln!();
    eprintln!("For library-only usage, you can use:");
    eprintln!("  cargo build --features minimal");
    eprintln!("  cargo build --features recommended");
    eprintln!();
    eprintln!("See the documentation for more information on feature presets.");
    std::process::exit(1);
}
