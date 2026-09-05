// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 审计(tests/e2e/audit_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.8):
//! - AUD-11 多线程并发 write 同一日志文件:行数 = 写入数,每行均为合法 JSON(无交错损坏)
//!
//! AUD-01…10 已有覆盖(tests/security/audit.rs);
//! CCY-06(8 线程压力档)固化于 concurrency_e2e.rs。

use confers::audit::AuditWriter;
use std::collections::HashSet;

#[test]
fn aud11_concurrent_writes_produce_one_intact_line_each() {
    let dir = tempfile::tempdir().unwrap();
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();

    const THREADS: usize = 8;
    const WRITES_PER_THREAD: usize = 25;
    const TOTAL: usize = THREADS * WRITES_PER_THREAD;

    let writer = std::sync::Arc::new(writer);
    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let writer = std::sync::Arc::clone(&writer);
            std::thread::spawn(move || {
                for i in 0..WRITES_PER_THREAD {
                    writer
                        .log_load(&format!("source-t{t}-i{i}"))
                        .expect("best-effort load event must write");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("writer thread must not panic");
    }

    // 允许落盘完成。
    std::thread::sleep(std::time::Duration::from_millis(200));

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir.path())
        .expect("log dir readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    assert!(!files.is_empty(), "audit log file must exist");

    let mut total_lines = 0usize;
    let mut seen_sources = HashSet::new();
    files.sort();
    for file in &files {
        let content = std::fs::read_to_string(file).expect("log file readable");
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            let json: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("every line must be valid JSON, got {e}: {line}"));
            assert!(
                json.get("LoadSuccess").is_some(),
                "line must be a LoadSuccess event: {line}"
            );
            assert!(
                line.contains("LoadSuccess"),
                "expected load events, got: {line}"
            );
            total_lines += 1;
            seen_sources.insert(line.to_string());
        }
    }

    assert_eq!(
        total_lines, TOTAL,
        "each of {TOTAL} concurrent writes must land as its own intact line"
    );
    assert_eq!(
        seen_sources.len(),
        TOTAL,
        "no interleaving may corrupt or duplicate records"
    );
}
