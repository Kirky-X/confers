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

    // 1. Initial config
    let path = "examples/watch.toml";
    std::fs::write(path, "message = 'Hello, initial!'\ninterval = 1000")?;

    // 2. Load with watcher
    println!("Starting config watcher... (Ctrl+C to stop)");

    // Initial load
    let config = WatchConfig::load()?;
    println!("Initial message: {}", config.message);

    // 3. Monitor changes
    let mut last_message = config.message.clone();

    // In a real app, you might use a channel or a callback
    // Here we just poll for demonstration
    for i in 1..=5 {
        println!("\n[Iteration {}] Change {} and wait...", i, path);

        // Simulate external change
        let new_message = format!("Hello, change {}!", i);
        std::fs::write(
            path,
            format!("message = '{}'\ninterval = 1000", new_message),
        )?;

        // Wait for debounce and file system
        std::thread::sleep(Duration::from_millis(500));

        // Check if changed
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
