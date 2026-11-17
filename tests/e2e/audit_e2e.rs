// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: 审计(tests/e2e/audit_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.8):
//! - AUD-11 多线程并发 write 同一日志文件:事件行数 = 写入数,每行均为合法 JSON(无交错损坏)
//! - AUD-12 rc.3 HMAC 链:并发写入后整条链可被 verify_audit_chain 校验通过
//!
//! AUD-01…10 已有覆盖(tests/security/audit.rs);
//! CCY-06(8 线程压力档)固化于 concurrency_e2e.rs。

use confers::audit::{AuditWriter, verify_audit_chain, verify_audit_chain_report};
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
    files.sort();

    let mut total_events = 0usize;
    let mut seen_sources = HashSet::new();
    for file in &files {
        let content = std::fs::read_to_string(file).expect("log file readable");
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            let json: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("every line must be valid JSON, got {e}: {line}"));
            // 链头/链尾元数据行(record 字段)不是事件行。
            if json.get("record").is_some() {
                continue;
            }
            assert!(
                json.get("LoadSuccess").is_some(),
                "event line must be a LoadSuccess event: {line}"
            );
            assert!(
                json.get("hmac").is_some() && json.get("prev_hash").is_some(),
                "event line must carry HMAC chain metadata: {line}"
            );
            total_events += 1;
            seen_sources.insert(line.to_string());
        }
    }

    assert_eq!(
        total_events, TOTAL,
        "each of {TOTAL} concurrent writes must land as its own intact event line"
    );
    assert_eq!(
        seen_sources.len(),
        TOTAL,
        "no interleaving may corrupt or duplicate records"
    );

    // 并发写入后整条 HMAC 链必须可校验。
    for file in &files {
        assert!(
            verify_audit_chain(file).expect("chain verification must not IO-error"),
            "chain in {} must verify after concurrent writes",
            file.display()
        );
    }
}

#[test]
fn report_verification_locates_tamper_after_writer_roundtrip() {
    // 写链 → report 版验证完好 → 篡改中间事件一个字节 → 断点定位在 2 且无 gap
    // （篡改不产生 gap，删除才产生）。
    let dir = tempfile::tempdir().unwrap();
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();
    for source in ["src-a", "src-b", "src-c"] {
        writer.log_load(source).expect("event must write");
    }
    drop(writer); // 追加链尾并落盘。

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir.path())
        .expect("log dir readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    assert!(!files.is_empty(), "audit log file must exist");
    files.sort();
    let path = &files[0];

    // 完好链：chain_intact 且全部事件 HMAC 有效。
    let report = verify_audit_chain_report(path).expect("report must not IO-error");
    assert!(report.chain_intact, "freshly written chain must be intact");
    assert_eq!(report.total_entries, 3);
    assert_eq!(report.hmac_valid_count, 3);

    // 篡改中间事件行的一个字节。
    let content = std::fs::read_to_string(path).expect("log file readable");
    let lines: Vec<&str> = content.lines().collect();
    let event_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()
                .and_then(|v| {
                    let obj = v.as_object()?;
                    Some(obj.contains_key("hmac") && !obj.contains_key("record"))
                })
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();
    assert_eq!(event_lines.len(), 3);
    let tampered = lines[event_lines[1]].replace("src-b", "src-X");
    assert_ne!(
        tampered, lines[event_lines[1]],
        "tamper must change the line"
    );
    let rewritten: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == event_lines[1] {
                tampered.clone()
            } else {
                (*l).to_string()
            }
        })
        .collect();
    std::fs::write(path, rewritten.join("\n") + "\n").expect("log file writable");

    // 篡改后：断点定位在事件 2，且不产生删除 gap。
    let report = verify_audit_chain_report(path).expect("report must not IO-error");
    assert!(!report.chain_intact, "tampered chain must not be intact");
    assert_eq!(report.first_broken_at, Some(2));
    assert!(
        report.gaps.is_empty(),
        "tampering must not produce deletion gaps: {:?}",
        report.gaps
    );
}
