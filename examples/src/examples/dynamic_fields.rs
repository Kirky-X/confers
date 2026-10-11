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
use confers::i18n::{tr, tr_args};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("{}\n", tr("example-dynamic-fields-start"));

    demo_basic_usage()?;
    demo_callback_guard()?;
    demo_builder_pattern()?;
    demo_performance_comparison()?;
    demo_multiple_callbacks()?;

    tracing::info!("\n{}", tr("example-dynamic-fields-all-completed"));
    Ok(())
}

fn demo_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("{}\n", tr("example-dynamic-fields-basic-demo"));

    let field = DynamicField::new(100u32);
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-initial-value",
            &[("value", field.get().to_string())]
        )
    );

    let value = field.get();
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-lock-free-read",
            &[("value", value.to_string())]
        )
    );

    field.update(200);
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-after-update",
            &[("value", field.get().to_string())]
        )
    );

    Ok(())
}

fn demo_callback_guard() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n{}\n", tr("example-dynamic-fields-callback-guard-demo"));

    let field = DynamicField::new(tr("example-dynamic-fields-initial-payload"));

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callbacks-before",
            &[("count", field.callback_count().to_string())]
        )
    );

    let _guard = field.on_change(move |new_val| {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        tracing::info!(
            "{}",
            tr_args(
                "example-dynamic-fields-callback-fired",
                &[("value", new_val.to_string())]
            )
        );
    });

    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callbacks-after",
            &[("count", field.callback_count().to_string())]
        )
    );

    field.update(tr("example-dynamic-fields-first-update"));
    field.update(tr("example-dynamic-fields-second-update"));

    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-times",
            &[("count", call_count.load(Ordering::SeqCst).to_string())]
        )
    );

    drop(_guard);
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callbacks-after-drop",
            &[("count", field.callback_count().to_string())]
        )
    );

    Ok(())
}

fn demo_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n{}\n", tr("example-dynamic-fields-builder-demo"));

    let field: DynamicField<i64> = DynamicField::builder().initial(42).build();
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-builder-value",
            &[("value", field.get().to_string())]
        )
    );

    let default_field: DynamicField<u32> = DynamicField::default();
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-default-value",
            &[("value", default_field.get().to_string())]
        )
    );

    Ok(())
}

fn demo_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n{}\n", tr("example-dynamic-fields-perf-demo"));

    const ITERATIONS: usize = 1_000_000;

    let lock_free_field = DynamicField::new(0u64);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = lock_free_field.get();
    }
    let lock_free_time = start.elapsed();
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-lock-free-time",
            &[
                ("iterations", ITERATIONS.to_string()),
                ("duration", format!("{lock_free_time:?}")),
            ]
        )
    );

    let rwlock_field = std::sync::RwLock::new(0u64);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _read_guard = rwlock_field.read().unwrap();
    }
    let rwlock_time = start.elapsed();
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-rwlock-time",
            &[
                ("iterations", ITERATIONS.to_string()),
                ("duration", format!("{rwlock_time:?}")),
            ]
        )
    );

    let speedup = rwlock_time.as_nanos() as f64 / lock_free_time.as_nanos() as f64;
    tracing::info!(
        "{}\n",
        tr_args(
            "example-dynamic-fields-speedup",
            &[("speedup", format!("{speedup:.2}"))]
        )
    );

    Ok(())
}

fn demo_multiple_callbacks() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("{}\n", tr("example-dynamic-fields-multi-callback-demo"));

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

    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-current-callbacks",
            &[("count", field.callback_count().to_string())]
        )
    );

    field.update(10);

    tracing::info!("{}", tr("example-dynamic-fields-after-update-10"));
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-a-value",
            &[("value", counter_a.load(Ordering::SeqCst).to_string())]
        )
    );
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-b-value",
            &[("value", counter_b.load(Ordering::SeqCst).to_string())]
        )
    );
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-c-value",
            &[("value", counter_c.load(Ordering::SeqCst).to_string())]
        )
    );

    drop(_guard_b);
    tracing::info!(
        "\n{}",
        tr_args(
            "example-dynamic-fields-callbacks-after-guard-b",
            &[("count", field.callback_count().to_string())]
        )
    );

    field.update(5);

    tracing::info!("\n{}", tr("example-dynamic-fields-after-update-5"));
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-a-value",
            &[("value", counter_a.load(Ordering::SeqCst).to_string())]
        )
    );
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-b-value",
            &[("value", counter_b.load(Ordering::SeqCst).to_string())]
        )
    );
    tracing::info!(
        "{}",
        tr_args(
            "example-dynamic-fields-callback-c-value",
            &[("value", counter_c.load(Ordering::SeqCst).to_string())]
        )
    );

    Ok(())
}
