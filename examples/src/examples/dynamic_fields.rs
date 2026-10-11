// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 动态字段示例 - 运行时配置更新
//!
//! 本示例展示如何使用 DynamicField 实现运行时配置更新：
//! - 无锁读取优化
//! - 回调通知机制
//! - Guard 自动清理
//! - 性能对比演示

use confers::dynamic::DynamicField;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("=== DynamicField example start ===\n");

    demo_basic_usage()?;
    demo_callback_guard()?;
    demo_builder_pattern()?;
    demo_performance_comparison()?;
    demo_multiple_callbacks()?;

    tracing::info!("\n=== All examples completed ===");
    Ok(())
}

fn demo_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("--- 1. Basic usage demo ---\n");

    let field = DynamicField::new(100u32);
    tracing::info!("Create DynamicField, initial value: {}", field.get());

    let value = field.get();
    tracing::info!("Lock-free read: {}", value);

    field.update(200);
    tracing::info!("Value after update: {}", field.get());

    Ok(())
}

fn demo_callback_guard() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 2. CallbackGuard lifecycle management ---\n");

    let field = DynamicField::new("initial value".to_string());

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    tracing::info!(
        "Callback count before registering: {}",
        field.callback_count()
    );

    let _guard = field.on_change(move |new_val| {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        tracing::info!("[callback fired] value changed to: {}", new_val);
    });

    tracing::info!(
        "Callback count after registering: {}",
        field.callback_count()
    );

    field.update("first update".to_string());
    field.update("second update".to_string());

    tracing::info!(
        "Callback fired times: {}",
        call_count.load(Ordering::SeqCst)
    );

    drop(_guard);
    tracing::info!(
        "Callback count after dropping the guard: {}",
        field.callback_count()
    );

    Ok(())
}

fn demo_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 3. Builder pattern ---\n");

    let field: DynamicField<i64> = DynamicField::builder().initial(42).build();
    tracing::info!("Field value created with Builder: {}", field.get());

    let default_field: DynamicField<u32> = DynamicField::default();
    tracing::info!("Field value created with Default: {}", default_field.get());

    Ok(())
}

fn demo_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 4. Performance comparison demo ---\n");

    const ITERATIONS: usize = 1_000_000;

    let lock_free_field = DynamicField::new(0u64);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = lock_free_field.get();
    }
    let lock_free_time = start.elapsed();
    tracing::info!(
        "Lock-free read (DynamicField) {} iterations took: {:?}",
        ITERATIONS,
        lock_free_time
    );

    let rwlock_field = std::sync::RwLock::new(0u64);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _read_guard = rwlock_field.read().unwrap();
    }
    let rwlock_time = start.elapsed();
    tracing::info!(
        "RwLock read {} iterations took: {:?}",
        ITERATIONS,
        rwlock_time
    );

    let speedup = rwlock_time.as_nanos() as f64 / lock_free_time.as_nanos() as f64;
    tracing::info!("Lock-free reads are {:.2}x faster than RwLock\n", speedup);

    Ok(())
}

fn demo_multiple_callbacks() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("--- 5. Multiple callbacks listening at once ---\n");

    let field = DynamicField::new(0u32);

    let counter_a = Arc::new(AtomicUsize::new(0));
    let counter_b = Arc::new(AtomicUsize::new(0));
    let counter_c = Arc::new(AtomicUsize::new(0));

    let c_a = counter_a.clone();
    let _guard_a = field.on_change(move |&val| {
        c_a.fetch_add(val as usize, Ordering::SeqCst);
    });

    let c_b2 = counter_b.clone();
    let _guard_b = field.on_change(move |&val| {
        c_b2.fetch_add(val as usize, Ordering::SeqCst);
    });

    let c_c2 = counter_c.clone();
    let _guard_c = field.on_change(move |&val| {
        c_c2.fetch_add(val as usize, Ordering::SeqCst);
    });

    tracing::info!("Current callback count: {}", field.callback_count());

    field.update(10);

    tracing::info!("After updating the value to 10:");
    tracing::info!(
        "  callback A accumulated value: {}",
        counter_a.load(Ordering::SeqCst)
    );
    tracing::info!(
        "  callback B accumulated value: {}",
        counter_b.load(Ordering::SeqCst)
    );
    tracing::info!(
        "  callback C accumulated value: {}",
        counter_c.load(Ordering::SeqCst)
    );

    drop(_guard_b);
    tracing::info!(
        "\nCallback count after dropping guard_b: {}",
        field.callback_count()
    );

    field.update(5);

    tracing::info!("\nAfter updating the value to 5:");
    tracing::info!(
        "  callback A accumulated value: {}",
        counter_a.load(Ordering::SeqCst)
    );
    tracing::info!(
        "  callback B accumulated value: {}",
        counter_b.load(Ordering::SeqCst)
    );
    tracing::info!(
        "  callback C accumulated value: {}",
        counter_c.load(Ordering::SeqCst)
    );

    Ok(())
}
