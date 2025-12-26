// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use clap::{Parser, Subcommand};
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
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "confers")]
#[command(about = "Configuration management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate configuration template
    Generate {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Template level (minimal, full)
        #[arg(short, long, default_value = "full")]
        level: String,
    },
    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: String,

        /// Output level (minimal, full, documentation)
        #[arg(short, long, default_value = "full")]
        level: String,
    },
    /// Diff two configuration files
    Diff {
        /// First file
        file1: String,
        /// Second file
        file2: String,

        /// Output style (unified, context, normal, side-by-side, strict)
        #[arg(short, long)]
        style: Option<String>,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: String,
    },
    /// Encrypt a value
    Encrypt {
        /// Value to encrypt
        value: String,

        /// Encryption key (Base64, 32 bytes). If not provided, uses CONFERS_ENCRYPTION_KEY env var.
        #[arg(short, long)]
        key: Option<String>,
    },
    /// Interactive configuration wizard
    Wizard {
        /// Skip interactive prompts and use default values
        #[arg(long)]
        non_interactive: bool,
    },
    /// Key management operations
    #[command(subcommand)]
    Key(#[command(subcommand)] confers::commands::key::KeySubcommand),
}

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
                // Use default values in non-interactive mode
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
