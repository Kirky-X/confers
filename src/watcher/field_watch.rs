// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Field-level hot-reload subscription for derived configuration structs.
//!
//! The `#[config(watch = true)]` field attribute makes the derive macro
//! generate a `field_watcher(rx)` constructor for [`StructFieldWatcher`]:
//! given the config watch channel it reports which of the subscribed fields
//! actually changed whenever a new configuration is published.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::watch;

use crate::types::ConfigValue;

/// Watch receiver for configuration snapshots (`tokio::sync::watch` based).
///
/// Re-exported so generated code (and users) can name the receiver type
/// without depending on tokio directly.
pub type WatchReceiver<T> = watch::Receiver<Arc<T>>;

/// Extracts a field's value from a configuration snapshot.
pub type FieldExtractor<T> = Box<dyn Fn(&T) -> ConfigValue + Send + Sync>;

/// Field-level change observer for arbitrary configuration structs.
///
/// Unlike [`crate::dynamic::FieldWatcher`] (which reads keys through
/// `ConfigProvider`), this observer works on typed structs: field values are
/// extracted with generated closures, so no trait has to be implemented by
/// the configuration struct.
pub struct StructFieldWatcher<T: Clone + Send + Sync + 'static> {
    rx: watch::Receiver<Arc<T>>,
    fields: Vec<(Arc<str>, FieldExtractor<T>)>,
    last: HashMap<Arc<str>, ConfigValue>,
}

impl<T: Clone + Send + Sync + 'static> StructFieldWatcher<T> {
    /// Create a watcher over `rx` for `fields` (name + extractor pairs).
    ///
    /// The baseline is seeded from the current snapshot so the initial
    /// value is never reported as a change.
    pub fn new(rx: watch::Receiver<Arc<T>>, fields: Vec<(Arc<str>, FieldExtractor<T>)>) -> Self {
        let mut last = HashMap::new();
        let current = rx.borrow().clone();
        for (name, extract) in &fields {
            last.insert(name.clone(), extract(&current));
        }
        Self { rx, fields, last }
    }

    /// Wait until any subscribed field changes.
    ///
    /// Returns the new snapshot together with the names of the fields whose
    /// values differ from the previously observed ones. Returns `None` once
    /// every sender of the channel has been dropped (the configuration
    /// source is gone).
    pub async fn changed(&mut self) -> Option<(Arc<T>, Vec<Arc<str>>)> {
        loop {
            if self.rx.changed().await.is_err() {
                return None;
            }
            let snapshot = self.rx.borrow().clone();
            let changed: Vec<Arc<str>> = self
                .fields
                .iter()
                .filter(|(name, extract)| {
                    let new_val = extract(&snapshot);
                    self.last.get(name) != Some(&new_val)
                })
                .map(|(name, _)| name.clone())
                .collect();

            if !changed.is_empty() {
                for (name, extract) in &self.fields {
                    self.last.insert(name.clone(), extract(&snapshot));
                }
                return Some((snapshot, changed));
            }
        }
    }

    /// The list of subscribed field names.
    pub fn watched_fields(&self) -> Vec<Arc<str>> {
        self.fields.iter().map(|(name, _)| name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[derive(Debug, Clone, PartialEq)]
    struct Cfg {
        host: String,
        port: u16,
    }

    fn watcher(rx: watch::Receiver<Arc<Cfg>>) -> StructFieldWatcher<Cfg> {
        StructFieldWatcher::new(
            rx,
            vec![
                (
                    "host".into(),
                    Box::new(|c: &Cfg| ConfigValue::string(c.host.clone())) as FieldExtractor<Cfg>,
                ),
                (
                    "port".into(),
                    Box::new(|c: &Cfg| ConfigValue::uint(c.port as u64)) as FieldExtractor<Cfg>,
                ),
            ],
        )
    }

    #[tokio::test]
    async fn reports_only_actually_changed_fields() {
        let (tx, rx) = watch::channel(Arc::new(Cfg {
            host: "a".into(),
            port: 1,
        }));
        let mut w = watcher(rx);

        tx.send(Arc::new(Cfg {
            host: "a".into(),
            port: 2,
        }))
        .unwrap();
        let (cfg, changed) = timeout(Duration::from_millis(200), w.changed())
            .await
            .expect("timeout")
            .expect("channel open");
        assert_eq!(changed, vec![Arc::<str>::from("port")]);
        assert_eq!(cfg.port, 2);

        // Unchanged snapshot -> loop continues, not reported.
        tx.send(Arc::new(Cfg {
            host: "a".into(),
            port: 2,
        }))
        .unwrap();
        tx.send(Arc::new(Cfg {
            host: "b".into(),
            port: 2,
        }))
        .unwrap();
        let (_, changed) = timeout(Duration::from_millis(200), w.changed())
            .await
            .expect("timeout")
            .expect("channel open");
        assert_eq!(changed, vec![Arc::<str>::from("host")]);
    }

    #[tokio::test]
    async fn returns_none_when_channel_closes() {
        let (tx, rx) = watch::channel(Arc::new(Cfg {
            host: "a".into(),
            port: 1,
        }));
        let mut w = watcher(rx);
        drop(tx);
        let result = w.changed().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn initial_value_is_not_reported_as_change() {
        let (tx, rx) = watch::channel(Arc::new(Cfg {
            host: "a".into(),
            port: 1,
        }));
        let mut w = watcher(rx);
        // Sending the identical value must not wake `changed` with a diff.
        tx.send(Arc::new(Cfg {
            host: "a".into(),
            port: 1,
        }))
        .unwrap();
        tx.send(Arc::new(Cfg {
            host: "z".into(),
            port: 1,
        }))
        .unwrap();
        let (_, changed) = timeout(Duration::from_millis(200), w.changed())
            .await
            .expect("timeout")
            .expect("channel open");
        assert_eq!(changed, vec![Arc::<str>::from("host")]);
        assert_eq!(
            w.watched_fields(),
            vec![Arc::<str>::from("host"), Arc::<str>::from("port")]
        );
    }
}
