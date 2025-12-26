use crate::error::ConfigError;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub struct CompletionsCommand;

impl CompletionsCommand {
    pub fn execute<C: CommandFactory>(shell: &str) -> Result<(), ConfigError> {
        let shell_enum = match shell {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
                return Err(ConfigError::FormatDetectionFailed(format!(
                    "Unsupported shell: {}",
                    shell
                )))
            }
        };

        let mut cmd = C::command();
        generate(shell_enum, &mut cmd, "confers", &mut io::stdout());

        Ok(())
    }
}
