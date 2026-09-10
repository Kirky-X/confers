// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Internal tracing facade (`tracing` feature).
//!
//! Critical-path instrumentation (load / reload / decrypt / remote fetch)
//! follows this shape at the call site:
//!
//! ```ignore
//! // Span: literal name required by the macro, two cfg-gated lines.
//! #[cfg(feature = "tracing")]
//! let load_span = tracing::info_span!("confers.load");
//! #[cfg(feature = "tracing")]
//! let _load_guard = load_span.enter();
//! ```
//!
//! Events go through [`event`], callable unconditionally: with the feature
//! disabled it compiles to a no-op, so the default feature set carries zero
//! tracing cost and no dependency. With the feature enabled, spans and events
//! flow into whatever `tracing` subscriber the application installed — fully
//! parallel to the [`crate::metrics`] backend, never instead of it.

/// Emit a structured event with `key=value` fields (no-op without the
/// `tracing` feature).
#[allow(unused_variables)]
pub(crate) fn event(name: &'static str, fields: &[(&'static str, &str)]) {
    #[cfg(feature = "tracing")]
    {
        tracing::event!(
            tracing::Level::INFO,
            event = name,
            fields = %fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "tracing")]
    use std::sync::{Arc, Mutex};

    /// Minimal capturing subscriber: records span names and event counts.
    #[cfg(feature = "tracing")]
    struct RecordingSubscriber {
        spans: Arc<Mutex<Vec<&'static str>>>,
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    #[cfg(feature = "tracing")]
    impl tracing::Subscriber for RecordingSubscriber {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::Id {
            let name = span.metadata().name();
            let mut spans = self.spans.lock().unwrap();
            spans.push(name);
            tracing::Id::from_u64(spans.len() as u64)
        }

        fn record(&self, _: &tracing::Id, _: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _: &tracing::Id, _: &tracing::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            self.events.lock().unwrap().push(event.metadata().name());
        }

        fn enter(&self, _: &tracing::Id) {}

        fn exit(&self, _: &tracing::Id) {}
    }

    /// Smoke test: a critical-path span and an event reach the subscribed
    /// sink through the exact call-site pattern documented above.
    #[test]
    #[cfg(feature = "tracing")]
    fn spans_and_events_reach_the_subscriber() {
        let spans = Arc::new(Mutex::new(Vec::new()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber = RecordingSubscriber {
            spans: Arc::clone(&spans),
            events: Arc::clone(&events),
        };
        tracing::subscriber::with_default(subscriber, || {
            // The pattern used at the four instrumented critical paths.
            let load_span = tracing::info_span!("confers.load");
            let _guard = load_span.enter();
            crate::telemetry::event("confers.load.completed", &[("ok", "true")]);
        });

        let seen_spans = spans.lock().unwrap().clone();
        let seen_events = events.lock().unwrap().clone();
        assert!(
            seen_spans.iter().any(|name| *name == "confers.load"),
            "spans recorded: {seen_spans:?}"
        );
        assert_eq!(seen_events.len(), 1, "events recorded: {seen_events:?}");
    }

    /// Without the feature the event helper must stay callable and no-op.
    #[test]
    fn event_helper_is_callable_without_feature() {
        crate::telemetry::event("confers.load.completed", &[]);
    }
}
