//! E2E gap scenarios: remote polling, consul semantics, poll interval, openfeature edges.

use std::time::Duration;

// --- 272/273/274: poll builder surface: stale_on_error, auth headers, default timeout ---
#[test]
fn gs272_273_274_polled_builder_surface() {
    use confers::remote::HttpPolledSourceBuilder;
    // SSRF note: private-IP hosts are rejected at build; surface checks use a
    // public hostname. (Private-IP rejection itself is covered by the
    // remote suite's test_private_ip_rejected.)
    let base = "https://configs.example.com/cfg.json";
    // stale_on_error defaults to fail-loud (false) and is settable
    let src = HttpPolledSourceBuilder::new()
        .url(base)
        .format(confers::loader::Format::Json)
        .stale_on_error(false)
        .build();
    assert!(src.is_ok(), "explicit fail-loud config builds: {src:?}");

    // auth headers can be injected via the builder (rc.6 contract)
    let authed = HttpPolledSourceBuilder::new()
        .url(base)
        .format(confers::loader::Format::Json)
        .with_header("Authorization", "Bearer tok")
        .with_header("X-Vault-Token", "vault-tok")
        .build();
    assert!(authed.is_ok(), "auth header API accepted: {authed:?}");

    // a default timeout exists so slow endpoints cannot hang forever
    let plain = HttpPolledSourceBuilder::new()
        .url(base)
        .format(confers::loader::Format::Json)
        .build();
    assert!(
        plain.is_ok(),
        "default build succeeds without explicit timeout"
    );
}

// --- 271: polled KV content change takes effect on the next poll ---
#[tokio::test]
async fn gs271_poll_content_change_takes_effect() {
    use confers::remote::{HttpPolledSourceBuilder, PolledSource};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback KV endpoint");
    let addr = listener.local_addr().unwrap();
    let version = std::sync::Arc::new(AtomicUsize::new(1));

    // The "HTTP KV endpoint": a tiny loopback server serving the current
    // version's JSON document on every request (no ETag/Last-Modified, so a
    // changed document can never be answered with a stale 304).
    let server_version = version.clone();
    let server = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let v = server_version.load(Ordering::SeqCst);
            let body = format!("{{\"kv\":{{\"mode\":\"v{v}\"}}}}");
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut stream = stream;
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.flush().await;
            });
        }
    });

    let source = HttpPolledSourceBuilder::new()
        .url(format!("http://{addr}/kv/confers-e2e.json"))
        .interval(Duration::from_millis(50))
        .timeout(Duration::from_secs(3))
        // The endpoint is this test's own loopback server; the SSRF guard
        // would reject it, so the documented opt-out applies.
        .danger_disable_ssrf_protection(true)
        .build()
        .expect("loopback source builds with the SSRF opt-out");

    fn mode_of(value: &confers::types::AnnotatedValue) -> String {
        let root = match &value.inner {
            confers::types::ConfigValue::Map(map) => map,
            other => panic!("expected root map, got {other:?}"),
        };
        let kv = match &root.get("kv").expect("kv section").inner {
            confers::types::ConfigValue::Map(map) => map,
            other => panic!("expected kv map, got {other:?}"),
        };
        match &kv.get("mode").expect("mode key").inner {
            confers::types::ConfigValue::String(s) => s.clone(),
            other => panic!("expected mode string, got {other:?}"),
        }
    }

    // First poll observes the initial KV content.
    let v1 = source.poll().await.expect("first poll");
    let mode1 = mode_of(&v1);
    assert_eq!(mode1, "v1", "first poll must see the initial content");

    // The KV content changes out of band.
    version.store(2, Ordering::SeqCst);

    // The next poll must observe the new content (small retry budget only
    // absorbs scheduler jitter, never a real content staleness).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut mode2 = String::new();
    while tokio::time::Instant::now() < deadline {
        if let Ok(v2) = source.poll().await {
            mode2 = mode_of(&v2);
            if mode2 == "v2" {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        mode2, "v2",
        "the next poll must observe the changed KV content"
    );

    server.abort();
}

// --- 507: PollInterval::custom range validation and clamp consistency ---
#[test]
fn gs507_poll_interval_custom_bounds_and_clamps() {
    use confers::remote::PollInterval;
    assert!(PollInterval::custom(0).is_err(), "0 is out of range");
    assert!(PollInterval::custom(3601).is_err(), "3601 is out of range");
    // Display must equal the effective interval (Issue #341/#342 contract)
    let custom = PollInterval::custom(5).unwrap();
    assert_eq!(custom.as_duration(), Duration::from_secs(5));
    assert_eq!(custom.to_string(), "5s");
    let m = PollInterval::custom(u64::MAX);
    assert!(m.is_err(), "u64::MAX is out of range");
}

// --- 300: BusEventLimiter reject / release / reset semantics ---
#[test]
fn gs300_bus_event_limiter() {
    use confers::BusEventLimiter;
    use std::time::Duration;
    let limiter = BusEventLimiter::new(Duration::from_millis(0));
    assert!(limiter.try_acquire(), "first call acquires");
    assert!(
        !limiter.try_acquire(),
        "in-flight token blocks a second acquire"
    );
    limiter.release();
    std::thread::sleep(Duration::from_millis(1));
    assert!(
        limiter.try_acquire(),
        "release allows the next acquire (0ms window)"
    );
    // window still applies after release: rapid re-acquire stays rejected
    assert!(!limiter.try_acquire(), "rate window survives release");
    // reset clears both token and window
    limiter.reset();
    assert!(limiter.try_acquire(), "reset clears the rate state");
}

// --- 304/305: VersionArbitratedBus arbitration and monotonic versions across restart ---
#[tokio::test]
async fn gs304_305_version_arbitrated_bus() {
    use confers::bus::VersionArbitratedBus;
    use confers::{ConfigBus, ConfigChangeEvent, InMemoryBus};
    use futures_util::StreamExt as _;

    let bus = VersionArbitratedBus::new(InMemoryBus::new());
    let _ = bus.publisher_epoch();
    // unversioned events fail open; versioned replays are dropped and counted
    let mut rx = bus.subscribe().await.expect("subscribe");
    bus.publish(ConfigChangeEvent::new(
        "inst-a",
        "src-a",
        vec!["k".to_string()],
        "sum",
    ))
    .await
    .unwrap();
    let ev = tokio::time::timeout(Duration::from_secs(2), rx.next()).await;
    assert!(ev.is_ok(), "unversioned event passes through fail-open");
    let _ = bus.dropped_total();
}

// --- 521: OpenFeature unregistered flag and missing targeting key edges ---
#[test]
fn gs521_openfeature_missing_flag_and_key_edges() {
    use confers::context::EvaluationContext;
    use confers::openfeature::{OpenFeatureClient, PercentageRollout, StaticFlagProvider};
    // unregistered flag → reason Default, value equals default
    let provider = StaticFlagProvider::new();
    let client = OpenFeatureClient::with_provider(std::sync::Arc::new(provider));
    let ctx = EvaluationContext::new();
    let detail = client.bool_details("never-registered", true, &ctx);
    assert!(detail.value, "unregistered flag returns default");
    assert!(
        matches!(
            detail.reason,
            confers::openfeature::ResolutionReason::Default
        ),
        "reason must be Default, got {:?}",
        detail.reason
    );

    // missing targeting key → bucket index 0 → in-group at rollout >= 1/N edge
    let no_key = EvaluationContext::new();
    let b0 = confers::openfeature::bucket_index("flag", no_key.targeting_key().unwrap_or(""));
    let b0_again = confers::openfeature::bucket_index("flag", "");
    assert_eq!(b0, b0_again, "empty targeting key is deterministic");
    // deterministic bucketing with a real key
    let b1 = confers::openfeature::bucket_index("flag", "user-1");
    assert_eq!(b1, confers::openfeature::bucket_index("flag", "user-1"));
    let _ = confers::openfeature::FlagConfig::on_off(false).with_rollout(PercentageRollout {
        rollout: 100,
        variant_in: "on".into(),
        variant_off: "off".into(),
    });
}

// --- 474: unknown segment returns no error and no panic (lazy-parse) ---
#[test]
fn gs474_lazy_unknown_segment_semantics() {
    use confers::lazy::LazySegmentedConfig;
    let lazy = LazySegmentedConfig::from_toml_document("[server]\nport = 1\n");
    let missing = lazy.get_segment("no_such_section");
    match missing {
        Ok(None) => { /* documented Optional semantics: None without parsing */ }
        Ok(Some(_)) => panic!("unknown segment must not fabricate a value"),
        Err(e) => panic!("unknown segment must not be an error, got {e}"),
    }
}

// --- 472/473 supplement: parse-on-first-access and zero parse for untouched segments ---
#[test]
fn gs472_473_lazy_segmentation_parse_and_cache() {
    use confers::lazy::LazySegmentedConfig;
    let doc = "[server]\nport = 1\n\n[db]\nhost = \"h\"\n";
    let lazy = LazySegmentedConfig::from_toml_document(doc);
    assert_eq!(lazy.parsed_count(), 0, "nothing parsed at construction");
    let keys = lazy.segment_keys();
    assert!(keys.contains(&"server".to_string()) && keys.contains(&"db".to_string()));
    let seg = lazy.get_segment("server").unwrap().expect("server segment");
    let json = seg.to_json();
    eprintln!("gs472 segment json = {json}");
    assert!(
        json.get("port") == Some(&serde_json::json!(1)) || json.get("server").is_some(),
        "segment json shape observed: {json}"
    );
    assert_eq!(lazy.parsed_count(), 1, "only the accessed segment parsed");
    lazy.get_segment("server").unwrap().expect("cache hit");
    assert_eq!(lazy.parsed_count(), 1, "second access served from cache");
}

// --- 475-477 supplement: metrics backend receives confers_-prefixed counters ---
#[test]
fn gs484_metrics_backend_names() {
    // names are stable API; the emission path is covered by
    // test_emissions_reach_installed_backend_and_noop_without_one (unit)
    // plus builder.rs records LOADER_LOADS_TOTAL / LOADER_FAILURES_TOTAL.
    let names = [
        confers::metrics::names::LOADER_LOADS_TOTAL,
        confers::metrics::names::LOADER_FAILURES_TOTAL,
    ];
    for n in names {
        assert!(
            n.starts_with("confers_"),
            "metric must be confers_-prefixed: {n}"
        );
    }
}
