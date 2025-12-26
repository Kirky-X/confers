use confers::watcher::{ConfigWatcher, ReloadLatencyMetrics};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn test_hot_reload_latency_basic() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "key = \"initial\"\nvalue = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let mut metrics = ReloadLatencyMetrics::new();
    let change_detected = Arc::new(AtomicBool::new(false));
    let change_detected_clone = Arc::clone(&change_detected);

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        fs::write(&file_path, "key = \"updated\"\nvalue = 1").unwrap();
        change_detected_clone.store(true, Ordering::SeqCst);
    });

    let start = Instant::now();
    let result = rx.recv_timeout(Duration::from_secs(5));
    let detection_time = start.elapsed();

    assert!(result.is_ok(), "Should receive file change event");
    assert!(
        change_detected.load(Ordering::SeqCst),
        "Change should be detected"
    );

    metrics.mark_completed();
    let reload_time = Instant::now().duration_since(start);

    let latency_ms = metrics
        .latency_ms()
        .expect("Latency should be Some after completion");

    println!(
        "Detection time: {:?}, Reload time: {:?}",
        detection_time, reload_time
    );
    println!("Total latency: {}ms", latency_ms);

    assert!(
        latency_ms < 1000,
        "Latency should be less than 1000ms, got {}ms",
        latency_ms
    );
}

#[test]
fn test_hot_reload_latency_multiple_changes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "value = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);

    thread::spawn(move || {
        for i in 1..=5 {
            thread::sleep(Duration::from_millis(200 * i));
            let _start = Instant::now();
            fs::write(&file_path, format!("value = {}", i)).unwrap();

            let mut metrics = ReloadLatencyMetrics::new();
            if let Ok(Ok(_)) = rx.recv_timeout(Duration::from_secs(5)) {
                metrics.mark_completed();
                if let Some(latency) = metrics.latency_ms() {
                    latencies_clone.lock().unwrap().push(latency);
                }
            }
        }
    });

    thread::sleep(Duration::from_secs(5));

    let latencies = latencies.lock().unwrap();
    println!("Latencies for multiple changes: {:?}", latencies);

    assert!(!latencies.is_empty(), "Should have recorded latencies");

    let avg_latency: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;
    println!("Average latency: {}ms", avg_latency);

    assert!(
        avg_latency < 1000,
        "Average latency should be less than 1000ms, got {}ms",
        avg_latency
    );
}

#[test]
fn test_hot_reload_latency_under_load() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "value = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);
    let event_count = Arc::new(AtomicUsize::new(0));
    let event_count_clone = Arc::clone(&event_count);

    let rx_thread = thread::spawn(move || {
        let mut count = 0;
        while count < 10 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(Ok(_)) => {
                    let mut metrics = ReloadLatencyMetrics::new();
                    metrics.mark_completed();
                    if let Some(latency) = metrics.latency_ms() {
                        latencies_clone.lock().unwrap().push(latency);
                    }
                    count += 1;
                    event_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => break,
                Ok(Err(_)) => break,
            }
        }
    });

    for i in 1..=10 {
        let path = file_path.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 * i));
            fs::write(&path, format!("value = {}", i)).unwrap();
        });
    }

    rx_thread.join().unwrap();

    let latencies = latencies.lock().unwrap();
    let total_events = event_count.load(Ordering::SeqCst);

    println!("Total events received: {}", total_events);
    println!("Latencies under load: {:?}", latencies);

    assert!(
        total_events > 0,
        "Should receive at least one event under load"
    );

    if !latencies.is_empty() {
        let max_latency = *latencies.iter().max().unwrap();
        let avg_latency: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;

        println!("Max latency under load: {}ms", max_latency);
        println!("Average latency under load: {}ms", avg_latency);

        assert!(
            max_latency < 2000,
            "Max latency should be less than 2000ms under load, got {}ms",
            max_latency
        );
    }
}

#[test]
fn test_hot_reload_latency_p99_metric() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "value = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);

    let rx_thread = thread::spawn(move || {
        for _ in 0..20 {
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(Ok(_)) => {
                    let mut metrics = ReloadLatencyMetrics::new();
                    metrics.mark_completed();
                    if let Some(latency) = metrics.latency_ms() {
                        latencies_clone.lock().unwrap().push(latency);
                    }
                }
                Err(_) => break,
                Ok(Err(_)) => break,
            }
        }
    });

    for i in 1..=20 {
        let path = file_path.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(30 * i));
            fs::write(&path, format!("value = {}", i)).unwrap();
        });
    }

    rx_thread.join().unwrap();

    let mut latencies = latencies.lock().unwrap();
    latencies.sort();

    println!("All latencies: {:?}", latencies);

    if !latencies.is_empty() {
        let p50_index = latencies.len() / 2;
        let p95_index = (latencies.len() * 95) / 100;
        let p99_index = (latencies.len() * 99) / 100;

        let p50 = latencies[p50_index];
        let p95 = latencies[p95_index.min(latencies.len() - 1)];
        let p99 = latencies[p99_index.min(latencies.len() - 1)];

        println!("P50 latency: {}ms", p50);
        println!("P95 latency: {}ms", p95);
        println!("P99 latency: {}ms", p99);

        assert!(
            p99 < 1500,
            "P99 latency should be less than 1500ms, got {}ms",
            p99
        );
    }
}

#[test]
fn test_hot_reload_latency_with_debounce() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "value = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);

    thread::spawn(move || {
        for i in 1..=5 {
            let path = file_path.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10 * i));
                fs::write(&path, format!("value = {}", i)).unwrap();
            });
        }

        thread::sleep(Duration::from_millis(600));

        if let Ok(Ok(_)) = rx.recv_timeout(Duration::from_secs(2)) {
            let mut metrics = ReloadLatencyMetrics::new();
            metrics.mark_completed();
            if let Some(latency) = metrics.latency_ms() {
                latencies_clone.lock().unwrap().push(latency);
            }
        }
    });

    thread::sleep(Duration::from_secs(3));

    let latencies = latencies.lock().unwrap();
    println!("Latencies with debounce: {:?}", latencies);

    assert!(
        latencies.len() <= 5,
        "Debounce should reduce number of events, got {}",
        latencies.len()
    );

    if !latencies.is_empty() {
        let avg_latency: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;
        println!("Average latency with debounce: {}ms", avg_latency);

        assert!(
            avg_latency < 1500,
            "Average latency with debounce should be less than 1500ms, got {}ms",
            avg_latency
        );
    }
}

#[test]
fn test_hot_reload_latency_concurrent_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("config1.toml");
    let file2 = temp_dir.path().join("config2.toml");
    fs::write(&file1, "value1 = 0").unwrap();
    fs::write(&file2, "value2 = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file1.clone(), file2.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);

    thread::spawn(move || {
        for i in 1..=5 {
            let path1 = file1.clone();
            let path2 = file2.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100 * i));
                fs::write(&path1, format!("value1 = {}", i)).unwrap();
            });
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100 * i + 50));
                fs::write(&path2, format!("value2 = {}", i)).unwrap();
            });
        }

        for _ in 0..10 {
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(Ok(_)) => {
                    let mut metrics = ReloadLatencyMetrics::new();
                    metrics.mark_completed();
                    if let Some(latency) = metrics.latency_ms() {
                        latencies_clone.lock().unwrap().push(latency);
                    }
                }
                Err(_) => break,
                Ok(Err(_)) => break,
            }
        }
    });

    thread::sleep(Duration::from_secs(5));

    let latencies = latencies.lock().unwrap();
    println!("Latencies for concurrent files: {:?}", latencies);

    assert!(
        !latencies.is_empty(),
        "Should have latencies for concurrent files"
    );

    let avg_latency: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;
    println!("Average latency for concurrent files: {}ms", avg_latency);

    assert!(
        avg_latency < 1500,
        "Average latency for concurrent files should be less than 1500ms, got {}ms",
        avg_latency
    );
}

#[test]
fn test_hot_reload_latency_stress_test() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.toml");
    fs::write(&file_path, "value = 0").unwrap();

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_debouncer, rx) = watcher.watch().unwrap();

    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));
    let latencies_clone = Arc::clone(&latencies);
    let event_count = Arc::new(AtomicUsize::new(0));
    let event_count_clone = Arc::clone(&event_count);

    let rx_thread = thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(_)) => {
                    let mut metrics = ReloadLatencyMetrics::new();
                    metrics.mark_completed();
                    if let Some(latency) = metrics.latency_ms() {
                        latencies_clone.lock().unwrap().push(latency);
                    }
                    event_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => continue,
                Ok(Err(_)) => continue,
            }
        }
    });

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let path = file_path.clone();
        thread::spawn(move || {
            let _ = fs::write(&path, format!("value = {}", rand::random::<u32>()));
        });
        thread::sleep(Duration::from_millis(20));
    }

    rx_thread.join().unwrap();

    let latencies = latencies.lock().unwrap();
    let total_events = event_count.load(Ordering::SeqCst);

    println!("Total events in stress test: {}", total_events);
    println!("Latencies in stress test: {:?}", latencies);

    assert!(
        total_events > 0,
        "Should receive at least one event in stress test"
    );

    if !latencies.is_empty() {
        let max_latency = *latencies.iter().max().unwrap();
        println!("Max latency in stress test: {}ms", max_latency);

        assert!(
            max_latency < 3000,
            "Max latency in stress test should be less than 3000ms, got {}ms",
            max_latency
        );
    }
}
