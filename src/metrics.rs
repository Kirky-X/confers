// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Process-wide [`MetricsBackend`] registry and the library's own critical-path
//! instrumentation.
//!
//! confers emits metrics on four critical paths — loader completion/failure,
//! watcher triggers, remote source fetch latency/errors, and secret decryption
//! errors — through the [`MetricsBackend`] port defined in
//! [`crate::interface`]. By default **no backend is installed** and every
//! emission is a cheap `None` check (behaviour identical to rc.2); downstream
//! applications opt in once per process with
//! [`set_metrics_backend`].
//!
//! ```
//! use std::sync::Arc;
//! use confers::types::NoOpMetrics;
//!
//! // Install a backend (returns the previously installed one, if any).
//! let previous = confers::metrics::set_metrics_backend(Arc::new(NoOpMetrics));
//! // ... run configuration loads / watchers / remote polls ...
//! confers::metrics::clear_metrics_backend();
//! ```

use crate::interface::MetricsBackend;
use std::sync::{Arc, RwLock};

/// Metric names emitted by confers. All names share the `confers_` prefix.
pub mod names {
    /// Counter: a configuration load (builder build) completed successfully.
    pub const LOADER_LOADS_TOTAL: &str = "confers_loader_loads_total";
    /// Counter: a configuration load failed.
    pub const LOADER_FAILURES_TOTAL: &str = "confers_loader_failures_total";
    /// Histogram: configuration load duration in seconds.
    pub const LOADER_DURATION_SECONDS: &str = "confers_loader_duration_seconds";
    /// Counter: file-watch events forwarded by a watcher (reload triggers).
    pub const WATCHER_EVENTS_TOTAL: &str = "confers_watcher_events_total";
    /// Histogram: remote source fetch latency in seconds.
    pub const REMOTE_FETCH_DURATION_SECONDS: &str = "confers_remote_fetch_duration_seconds";
    /// Counter: remote source fetch errors.
    pub const REMOTE_FETCH_ERRORS_TOTAL: &str = "confers_remote_fetch_errors_total";
    /// Counter: secret decryption failures.
    pub const SECRET_DECRYPT_ERRORS_TOTAL: &str = "confers_secret_decrypt_errors_total";
}

static METRICS_BACKEND: RwLock<Option<Arc<dyn MetricsBackend>>> = RwLock::new(None);

fn backend() -> Option<Arc<dyn MetricsBackend>> {
    METRICS_BACKEND
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Install the process-wide metrics backend, returning the previously
/// installed one (if any). Emissions before this call are discarded.
pub fn set_metrics_backend(backend: Arc<dyn MetricsBackend>) -> Option<Arc<dyn MetricsBackend>> {
    METRICS_BACKEND
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .replace(backend)
}

/// Remove the process-wide metrics backend, returning it (if any).
pub fn clear_metrics_backend() -> Option<Arc<dyn MetricsBackend>> {
    METRICS_BACKEND
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

/// The currently installed backend, if any (mainly for tests/diagnostics).
pub fn metrics_backend() -> Option<Arc<dyn MetricsBackend>> {
    backend()
}

/// Record a counter on the installed backend (no-op when none is installed).
pub(crate) fn record_counter(name: &str, labels: &[(&str, &str)]) {
    if let Some(backend) = backend() {
        backend.counter(name, labels);
    }
}

/// Record a histogram value on the installed backend (no-op when none is
/// installed).
pub(crate) fn record_histogram(name: &str, value: f64, labels: &[(&str, &str)]) {
    if let Some(backend) = backend() {
        backend.histogram(name, value, labels);
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Test-only [`MetricsBackend`] recording every emission in memory so
    //! instrumentation tests can assert on emitted metric names/counts.

    use crate::interface::MetricsBackend;
    use std::sync::{Arc, Mutex};

    /// Recording backend counting emissions per metric name.
    pub(crate) struct RecordingBackend {
        counters: Mutex<Vec<(String, Vec<(String, String)>)>>,
        histograms: Mutex<Vec<(String, f64, Vec<(String, String)>)>>,
    }

    impl Default for RecordingBackend {
        fn default() -> Self {
            Self {
                counters: Mutex::new(Vec::new()),
                histograms: Mutex::new(Vec::new()),
            }
        }
    }

    impl MetricsBackend for RecordingBackend {
        fn counter(&self, name: &str, labels: &[(&str, &str)]) {
            self.counters.lock().unwrap().push((
                name.to_string(),
                labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ));
        }

        fn histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
            self.histograms.lock().unwrap().push((
                name.to_string(),
                value,
                labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ));
        }
    }

    impl RecordingBackend {
        pub(crate) fn installed() -> Arc<Self> {
            let backend = Arc::new(Self::default());
            super::set_metrics_backend(backend.clone());
            backend
        }

        pub(crate) fn counter_count(&self, name: &str) -> usize {
            self.counters
                .lock()
                .unwrap()
                .iter()
                .filter(|(n, _)| n == name)
                .count()
        }

        pub(crate) fn histogram_count(&self, name: &str) -> usize {
            self.histograms
                .lock()
                .unwrap()
                .iter()
                .filter(|(n, _, _)| n == name)
                .count()
        }

        /// Labels of the first histogram recorded under `name`.
        pub(crate) fn first_histogram_labels(&self, name: &str) -> Vec<(String, String)> {
            self.histograms
                .lock()
                .unwrap()
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, _, labels)| labels.clone())
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::test_support::RecordingBackend;
    use serial_test::serial;
    use std::sync::Arc;

    #[test]
    #[serial]
    fn test_emissions_reach_installed_backend_and_noop_without_one() {
        // Without a backend: emissions are silent no-ops.
        clear_metrics_backend();
        record_counter(names::LOADER_LOADS_TOTAL, &[]);
        record_histogram(names::LOADER_DURATION_SECONDS, 1.0, &[]);
        assert!(metrics_backend().is_none());

        // With a backend: every emission lands with the confers_ prefix intact.
        let recorder = RecordingBackend::installed();

        record_counter(names::LOADER_LOADS_TOTAL, &[("result", "ok")]);
        record_counter(names::SECRET_DECRYPT_ERRORS_TOTAL, &[]);
        record_histogram(names::REMOTE_FETCH_DURATION_SECONDS, 0.25, &[("source", "http:x")]);

        clear_metrics_backend();

        assert_eq!(recorder.counter_count(names::LOADER_LOADS_TOTAL), 1);
        assert_eq!(
            recorder.counter_count(names::SECRET_DECRYPT_ERRORS_TOTAL),
            1
        );
        assert_eq!(
            recorder.histogram_count(names::REMOTE_FETCH_DURATION_SECONDS),
            1
        );
        assert_eq!(
            recorder.first_histogram_labels(names::REMOTE_FETCH_DURATION_SECONDS),
            vec![("source".to_string(), "http:x".to_string())]
        );

        // Cleanup removes the backend again.
        assert!(metrics_backend().is_none());
    }

    #[test]
    #[serial]
    fn test_set_returns_previous_backend() {
        clear_metrics_backend();
        let first: Arc<dyn MetricsBackend> = Arc::new(test_support::RecordingBackend::default());
        let second: Arc<dyn MetricsBackend> = Arc::new(test_support::RecordingBackend::default());

        assert!(set_metrics_backend(first.clone()).is_none());
        let replaced = set_metrics_backend(second.clone()).expect("previous backend returned");
        assert!(Arc::ptr_eq(&first, &replaced));

        clear_metrics_backend();
        assert!(metrics_backend().is_none());
    }
}
