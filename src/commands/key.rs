// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::key::KeyStorage;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::Rng;
use std::path::{Path, PathBuf};

pub struct KeyCommand;

impl KeyCommand {
    pub fn execute(
        subcommand: &KeySubcommand,
        _storage_dir: Option<&str>,
        master_key: Option<&str>,
    ) -> Result<(), ConfigError> {
        match subcommand {
            KeySubcommand::Generate {
                output,
                algorithm,
                size,
            } => {
                Self::generate_key(output, algorithm, *size)?;
            }
            KeySubcommand::Init {
                key_id,
                storage,
                rotate_interval,
            } => {
                let master_key = master_key.ok_or_else(|| {
                    ConfigError::RuntimeError("Master key required for init".to_string())
                })?;
                let master_key_bytes = Self::parse_master_key(master_key)?;
                let storage_path = Self::resolve_storage_path(storage)?;

                Self::init_key_ring(
                    &master_key_bytes,
                    key_id,
                    &storage_path,
                    rotate_interval.as_ref(),
                )?;
            }
            KeySubcommand::Rotate {
                key_id,
                storage,
                by,
                reason,
            } => {
                let master_key = master_key.ok_or_else(|| {
                    ConfigError::RuntimeError("Master key required for rotation".to_string())
                })?;
                let master_key_bytes = Self::parse_master_key(master_key)?;
                let storage_path = Self::resolve_storage_path(storage)?;

                Self::rotate_key(
                    &master_key_bytes,
                    key_id.as_deref(),
                    by.as_deref().unwrap_or("cli"),
                    reason.as_deref(),
                    &storage_path,
                )?;
            }
            KeySubcommand::List { storage, json } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::list_keys(&storage_path, *json)?;
            }
            KeySubcommand::Info {
                key_id,
                storage,
                json,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::key_info(key_id, &storage_path, *json)?;
            }
            KeySubcommand::Status { storage, json } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::rotation_status(&storage_path, *json)?;
            }
            KeySubcommand::Plan {
                target,
                key_id,
                storage,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::plan_rotation(*target, key_id.as_deref(), &storage_path)?;
            }
            KeySubcommand::Export {
                output,
                key_id,
                storage,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::export_keys(output, key_id.as_deref(), &storage_path)?;
            }
            KeySubcommand::Import {
                input,
                storage,
                new_master_key,
            } => {
                let new_key = new_master_key.as_ref().cloned().ok_or_else(|| {
                    ConfigError::RuntimeError("New master key required for import".to_string())
                });
                let new_key_bytes = Self::parse_master_key(&new_key?)?;
                let storage_path = Self::resolve_storage_path(storage)?;

                Self::import_keys(input, &storage_path, &new_key_bytes)?;
            }
            KeySubcommand::Backup { output, storage } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::backup_keys(output, &storage_path)?;
            }
            KeySubcommand::Restore {
                backup,
                storage,
                new_master_key,
            } => {
                let new_key = new_master_key.as_ref().cloned().ok_or_else(|| {
                    ConfigError::RuntimeError("New master key required for restore".to_string())
                });
                let new_key_bytes = Self::parse_master_key(&new_key?)?;
                let storage_path = Self::resolve_storage_path(storage)?;

                Self::restore_keys(backup, &storage_path, &new_key_bytes)?;
            }
            KeySubcommand::Deprecate {
                version,
                key_id,
                storage,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::deprecate_version(*version, key_id.as_deref(), &storage_path)?;
            }
            KeySubcommand::Cleanup {
                key_id,
                keep,
                storage,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::cleanup_old_keys(key_id.as_deref(), *keep, &storage_path)?;
            }
            KeySubcommand::Migrate {
                from_version,
                to_version,
                key_id,
                storage,
            } => {
                let storage_path = Self::resolve_storage_path(storage)?;
                Self::migrate_keys(*from_version, *to_version, key_id.as_deref(), &storage_path)?;
            }
        }

        Ok(())
    }

    fn generate_key(
        output: &Option<String>,
        algorithm: &str,
        size: u32,
    ) -> Result<(), ConfigError> {
        let key_bytes = match algorithm {
            "AES256" | "aes256" => {
                if size != 256 {
                    return Err(ConfigError::FormatDetectionFailed(
                        "AES key size must be 256 bits".to_string(),
                    ));
                }
                let mut key = [0u8; 32];
                let mut rng = rand::rng();
                rng.fill(&mut key);
                key
            }
            _ => {
                return Err(ConfigError::FormatDetectionFailed(format!(
                    "Unsupported algorithm: {}. Only AES256 is supported.",
                    algorithm
                )));
            }
        };

        let key_b64 = BASE64.encode(key_bytes);

        if let Some(output_path) = output {
            std::fs::write(output_path, &key_b64)
                .map_err(|e| ConfigError::IoError(format!("Failed to write key file: {}", e)))?;
            println!("Key generated and saved to: {}", output_path);
        } else {
            println!("{}", key_b64);
        }

        Ok(())
    }

    fn parse_master_key(key_str: &str) -> Result<[u8; 32], ConfigError> {
        let key_bytes = BASE64.decode(key_str).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid master key: {}", e))
        })?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::FormatDetectionFailed(
                "Master key must be 32 bytes (256 bits)".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    }

    fn resolve_storage_path(storage: &Option<String>) -> Result<PathBuf, ConfigError> {
        Ok(match storage {
            Some(path) => PathBuf::from(path),
            None => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home)
                    .join(".config")
                    .join("confers")
                    .join("keys")
            }
        })
    }

    fn init_key_ring(
        master_key: &[u8; 32],
        key_id: &str,
        storage_path: &PathBuf,
        rotate_interval: Option<&u32>,
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.clone())?;
        storage.set_master_key(master_key);

        storage.initialize_with_master_key(master_key, key_id.to_string(), "cli".to_string())?;

        if let Some(interval) = rotate_interval {
            storage
                .get_key_manager_mut()
                .set_rotation_interval(key_id, *interval)?;
            storage.save()?;
        }

        println!("Key ring '{}' initialized successfully", key_id);
        println!("Storage path: {:?}", storage_path);
        Ok(())
    }

    fn rotate_key(
        master_key: &[u8; 32],
        key_id: Option<&str>,
        rotated_by: &str,
        reason: Option<&str>,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.to_path_buf())?;
        storage.set_master_key(master_key);
        storage.load()?;

        let result = storage.get_key_manager_mut().rotate_key(
            master_key,
            key_id.map(|s| s.to_string()),
            rotated_by.to_string(),
            reason.map(|s| s.to_string()),
        )?;

        storage.save()?;

        println!("Key rotation successful:");
        println!("  Key ID: {}", result.key_id);
        println!("  Previous version: {}", result.previous_version);
        println!("  New version: {}", result.new_version);
        println!("  Re-encryption required: {}", result.reencryption_required);

        Ok(())
    }

    /// 列出存储中的所有密钥
    fn list_keys(storage_path: &Path, json_output: bool) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let keys = storage.get_key_manager().list_keys();

        if json_output {
            match serde_json::to_string_pretty(&keys) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Failed to serialize keys to JSON: {}", e);
                    return Err(ConfigError::SerializationError(e.to_string()));
                }
            }
        } else {
            println!("Key Rings:");
            println!("{}", "=".repeat(60));

            for key in &keys {
                println!("  Key ID: {}", key.key_id);
                println!("    Current Version: v{}", key.current_version);
                println!("    Total Versions: {}", key.total_versions);
                println!("    Active Versions: {}", key.active_versions);
                println!("    Deprecated: {}", key.deprecated_versions);
                println!("    Created: {}", format_timestamp(key.created_at));
                if let Some(last_rotated) = key.last_rotated_at {
                    println!("    Last Rotated: {}", format_timestamp(last_rotated));
                }
                println!();
            }
        }

        Ok(())
    }

    /// 获取特定密钥的信息
    fn key_info(key_id: &str, storage_path: &Path, json_output: bool) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let info = storage.get_key_manager().get_key_info(key_id)?;

        if json_output {
            match serde_json::to_string_pretty(&info) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Failed to serialize key info to JSON: {}", e);
                    return Err(ConfigError::SerializationError(e.to_string()));
                }
            }
        } else {
            println!("Key Information for '{}':", key_id);
            println!("{}", "=".repeat(60));
            println!("  Key ID: {}", info.key_id);
            println!("  Current Version: v{}", info.current_version);
            println!("  Total Versions: {}", info.total_versions);
            println!("  Active Versions: {}", info.active_versions);
            println!("  Deprecated Versions: {}", info.deprecated_versions);
            println!("  Created: {}", format_timestamp(info.created_at));
            if let Some(last_rotated) = info.last_rotated_at {
                println!("  Last Rotated: {}", format_timestamp(last_rotated));
            }
        }

        Ok(())
    }

    fn rotation_status(storage_path: &Path, json_output: bool) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let status_list = storage.get_key_manager().get_rotation_status();

        if json_output {
            match serde_json::to_string_pretty(&status_list) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Failed to serialize rotation status to JSON: {}", e);
                    return Err(ConfigError::SerializationError(e.to_string()));
                }
            }
        } else {
            println!("Key Rotation Status:");
            println!("{}", "=".repeat(60));

            for status in &status_list {
                println!("  Key ID: {}", status.key_id);
                println!("    Current Version: v{}", status.current_version);
                println!(
                    "    Rotation Interval: {} days",
                    status.rotation_interval_days
                );
                println!(
                    "    Last Rotation: {}",
                    format_timestamp(status.last_rotation)
                );
                println!(
                    "    Next Rotation: {}",
                    format_timestamp(status.next_rotation)
                );
                println!("    Days Until Rotation: {}", status.days_until_rotation);

                if status.is_overdue {
                    println!("    Status: OVERDUE ⚠️");
                } else {
                    println!("    Status: OK");
                }

                println!("    Auto-rotate: {}", status.auto_rotate);
                println!();
            }
        }

        Ok(())
    }

    fn plan_rotation(
        target: u32,
        key_id: Option<&str>,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let plan = storage
            .get_key_manager()
            .plan_rotation(target, key_id.map(|s| s.to_string()))?;

        println!("Rotation Plan for '{}':", plan.key_id);
        println!("{}", "=".repeat(60));
        println!("  Current Version: v{}", plan.current_version);
        println!("  Target Version: v{}", plan.target_version);
        println!("  Versions to Rotate: {:?}", plan.keys_to_rotate);
        println!("  Re-encryption Required: {}", plan.reencryption_required);
        println!();

        if plan.keys_to_rotate.is_empty() {
            println!("  No rotations needed.");
        } else {
            println!("  Rotation Steps:");
            for (i, version) in plan.keys_to_rotate.iter().enumerate() {
                println!("    {}. Rotate to v{}", i + 1, version);
            }
        }

        Ok(())
    }

    fn export_keys(
        output: &str,
        _key_id: Option<&str>,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let output_path = PathBuf::from(output);
        storage.export_keys(&output_path)?;

        println!("Keys exported to: {:?}", output_path);
        println!("Note: The exported file is encrypted. Keep it safe!");

        Ok(())
    }

    /// 从文件导入密钥
    fn import_keys(
        input: &str,
        storage_path: &PathBuf,
        new_master_key: &[u8; 32],
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.clone())?;
        let input_path = PathBuf::from(input);
        storage.import_keys(&input_path, new_master_key)?;

        println!("密钥导入成功");
        println!("存储路径: {:?}", storage_path);

        Ok(())
    }

    /// 创建密钥备份
    fn backup_keys(output: &str, storage_path: &Path) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let output_path = PathBuf::from(output);
        storage.backup(&output_path)?;

        println!("备份已创建: {:?}", output_path);

        Ok(())
    }

    /// 从备份恢复密钥
    fn restore_keys(
        backup: &str,
        storage_path: &PathBuf,
        new_master_key: &[u8; 32],
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.clone())?;
        let backup_path = PathBuf::from(backup);

        let master_key = new_master_key;
        storage.set_master_key(master_key);
        storage.load()?;

        println!("密钥已成功从备份恢复: {:?}", backup_path);
        println!("存储路径: {:?}", storage_path);

        Ok(())
    }

    fn deprecate_version(
        version: u32,
        key_id: Option<&str>,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.to_path_buf())?;
        let effective_key_id = key_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| storage.get_key_manager().get_default_key_id().to_string());

        storage
            .get_key_manager_mut()
            .deprecate_version(&effective_key_id, version)?;
        storage.save()?;

        println!(
            "Version v{} of key '{}' has been deprecated",
            version, effective_key_id
        );

        Ok(())
    }

    fn cleanup_old_keys(
        key_id: Option<&str>,
        keep: u32,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let mut storage = KeyStorage::new(storage_path.to_path_buf())?;
        let effective_key_id = key_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| storage.get_key_manager().get_default_key_id().to_string());

        let removed = storage
            .get_key_manager_mut()
            .cleanup_old_keys(&effective_key_id, keep)?;
        storage.save()?;

        println!(
            "Cleaned up {} old key versions for '{}' (keeping {})",
            removed, effective_key_id, keep
        );

        Ok(())
    }

    /// 规划密钥迁移
    fn migrate_keys(
        from_version: u32,
        to_version: u32,
        key_id: Option<&str>,
        storage_path: &Path,
    ) -> Result<(), ConfigError> {
        let storage = KeyStorage::new(storage_path.to_path_buf())?;
        let key_id = key_id.unwrap_or_else(|| storage.get_key_manager().get_default_key_id());

        println!("密钥 '{}' 从 v{} 到 v{} 的迁移计划:", key_id, from_version, to_version);
        println!("{}", "=".repeat(60));
        println!(
            "  此操作将使用 v{} 重新加密所有数据",
            from_version
        );
        println!("  使用新的密钥版本 v{}。", to_version);
        println!();
        println!("  步骤:");
        println!("    1. 获取旧密钥 (v{})", from_version);
        println!("    2. 获取新密钥 (v{})", to_version);
        println!("    3. 使用旧密钥解密所有数据");
        println!("    4. 使用新密钥重新加密所有数据");
        println!("    5. 更新元数据以反映迁移情况");
        println!();
        println!("  注意: 这是一个计划中的迁移，请在维护窗口期间谨慎执行。");

        Ok(())
    }
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum KeySubcommand {
    Generate {
        output: Option<String>,
        #[arg(short, long, default_value = "AES256")]
        algorithm: String,
        #[arg(short, long, default_value = "256")]
        size: u32,
    },
    Init {
        key_id: String,
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(short, long)]
        rotate_interval: Option<u32>,
    },
    Rotate {
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(short, long)]
        by: Option<String>,
        #[arg(short, long)]
        reason: Option<String>,
    },
    List {
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Info {
        key_id: String,
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Status {
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Plan {
        target: u32,
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long)]
        storage: Option<String>,
    },
    Export {
        output: String,
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long)]
        storage: Option<String>,
    },
    Import {
        input: String,
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(short, long)]
        new_master_key: Option<String>,
    },
    Backup {
        output: String,
        #[arg(short, long)]
        storage: Option<String>,
    },
    Restore {
        backup: String,
        #[arg(short, long)]
        storage: Option<String>,
        #[arg(short, long)]
        new_master_key: Option<String>,
    },
    Deprecate {
        version: u32,
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long)]
        storage: Option<String>,
    },
    Cleanup {
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long, default_value = "3")]
        keep: u32,
        #[arg(short, long)]
        storage: Option<String>,
    },
    Migrate {
        from_version: u32,
        to_version: u32,
        #[arg(short, long)]
        key_id: Option<String>,
        #[arg(short, long)]
        storage: Option<String>,
    },
}

fn format_timestamp(ts: u64) -> String {
    let dt = chrono::DateTime::from_timestamp(ts as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(chrono::Utc::now));
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
