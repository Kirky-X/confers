use confers::dynamic::DynamicField;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("=== DynamicField 示例启动 ===\n");

    demo_basic_usage()?;
    demo_callback_guard()?;
    demo_builder_pattern()?;
    demo_performance_comparison()?;
    demo_multiple_callbacks()?;

    tracing::info!("\n=== 所有示例运行完成 ===");
    Ok(())
}

fn demo_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("--- 1. 基础用法演示 ---\n");

    let field = DynamicField::new(100u32);
    tracing::info!("创建 DynamicField，初始值: {}", field.get());

    let value = field.get();
    tracing::info!("无锁读取: {}", value);

    field.update(200);
    tracing::info!("更新后值: {}", field.get());

    Ok(())
}

fn demo_callback_guard() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 2. CallbackGuard 生命周期管理 ---\n");

    let field = DynamicField::new("初始值".to_string());

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    tracing::info!("注册回调前回调数量: {}", field.callback_count());

    let _guard = field.on_change(move |new_val| {
        call_count_clone.fetch_add(1, Ordering::SeqCst);
        tracing::info!("[回调触发] 值变化为: {}", new_val);
    });

    tracing::info!("注册回调后回调数量: {}", field.callback_count());

    field.update("第一次更新".to_string());
    field.update("第二次更新".to_string());

    tracing::info!("回调触发次数: {}", call_count.load(Ordering::SeqCst));

    drop(_guard);
    tracing::info!("guard 销毁后回调数量: {}", field.callback_count());

    Ok(())
}

fn demo_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 3. Builder 模式 ---\n");

    let field: DynamicField<i64> = DynamicField::builder().initial(42).build();
    tracing::info!("使用 Builder 创建的字段值: {}", field.get());

    let default_field: DynamicField<u32> = DynamicField::default();
    tracing::info!("Default 创建的字段值: {}", default_field.get());

    Ok(())
}

fn demo_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("\n--- 4. 性能对比演示 ---\n");

    const ITERATIONS: usize = 1_000_000;

    let lock_free_field = DynamicField::new(0u64);
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = lock_free_field.get();
    }
    let lock_free_time = start.elapsed();
    tracing::info!(
        "无锁读取 (DynamicField) {} 次耗时: {:?}",
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
        "RwLock 读取 {} 次耗时: {:?}",
        ITERATIONS,
        rwlock_time
    );

    let speedup = rwlock_time.as_nanos() as f64 / lock_free_time.as_nanos() as f64;
    tracing::info!("无锁读取比 RwLock 快 {:.2} 倍\n", speedup);

    Ok(())
}

fn demo_multiple_callbacks() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("--- 5. 多个回调同时监听 ---\n");

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

    tracing::info!("当前回调数量: {}", field.callback_count());

    field.update(10);

    tracing::info!("更新值为 10 后:");
    tracing::info!("  回调 A 累计值: {}", counter_a.load(Ordering::SeqCst));
    tracing::info!("  回调 B 累计值: {}", counter_b.load(Ordering::SeqCst));
    tracing::info!("  回调 C 累计值: {}", counter_c.load(Ordering::SeqCst));

    drop(_guard_b);
    tracing::info!("\n销毁 guard_b 后回调数量: {}", field.callback_count());

    field.update(5);

    tracing::info!("\n更新值为 5 后:");
    tracing::info!("  回调 A 累计值: {}", counter_a.load(Ordering::SeqCst));
    tracing::info!("  回调 B 累计值: {}", counter_b.load(Ordering::SeqCst));
    tracing::info!("  回调 C 累计值: {}", counter_c.load(Ordering::SeqCst));

    Ok(())
}
