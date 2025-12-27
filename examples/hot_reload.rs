// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(format_detection = "Auto")]
pub struct WatchConfig {
    pub message: String,
    pub interval: u64,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. 初始配置
    let path = "examples/watch.toml";
    std::fs::write(path, "message = 'Hello, initial!'\ninterval = 1000")?;

    // 2. 加载并监控
    println!("Starting config watcher... (Ctrl+C to stop)");

    // 初始加载
    let config = WatchConfig::load()?;
    println!("Initial message: {}", config.message);

    // 3. 监控变化
    let mut last_message = config.message.clone();

    // 在实际应用中，您可能使用通道或回调
    // 这里我们只是轮询演示
    for i in 1..=5 {
        println!("\n[Iteration {}] Change {} and wait...", i, path);

        // 模拟外部变化
        let new_message = format!("Hello, change {}!", i);
        std::fs::write(
            path,
            format!("message = '{}'\ninterval = 1000", new_message),
        )?;

        // 等待防抖和文件系统
        std::thread::sleep(Duration::from_millis(500));

        // 检查是否变化
        let current_config = WatchConfig::load()?;
        if current_config.message != last_message {
            println!(
                ">>> Config changed! New message: {}",
                current_config.message
            );
            last_message = current_config.message.clone();
        } else {
            println!("No change detected yet...");
        }
    }

    Ok(())
}
