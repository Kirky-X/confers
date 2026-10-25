// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Watcher wrapper that normalizes the event kinds of the platform backend.
//!
//! macOS FSEvents reports `Create` for every file it mentions in a batch —
//! including a plain modification and even a deletion of a file that already
//! existed. `notify-debouncer-full` drops every later `Modify` for a path
//! whose queue starts with a `Create`, and discards a `Remove` for such a
//! path outright. A deletion would therefore never reach the consumer.
//!
//! Mapping `Create` onto `Modify` removes the ambiguity: the watcher reports
//! "this path changed", which is exactly the signal the consumer acts on.

use notify_debouncer_full::notify::event::{DataChange, ModifyKind};
use notify_debouncer_full::notify::{
    Config, Error, Event, EventHandler, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    WatcherKind,
};
use std::path::Path;

/// A [`notify::Watcher`] that reports creations as modifications.
pub struct CreateAsModifyWatcher {
    inner: RecommendedWatcher,
}

impl Watcher for CreateAsModifyWatcher {
    fn new<F: EventHandler>(mut event_handler: F, config: Config) -> Result<Self, Error> {
        let inner = RecommendedWatcher::new(
            move |res: std::result::Result<Event, Error>| {
                let event = match res {
                    Ok(mut event) => {
                        if matches!(event.kind, EventKind::Create(_)) {
                            event.kind = EventKind::Modify(ModifyKind::Data(DataChange::Content));
                        }
                        Ok(event)
                    }
                    Err(e) => Err(e),
                };
                event_handler.handle_event(event);
            },
            config,
        )?;
        Ok(Self { inner })
    }

    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> Result<(), Error> {
        self.inner.watch(path, recursive_mode)
    }

    fn unwatch(&mut self, path: &Path) -> Result<(), Error> {
        self.inner.unwatch(path)
    }

    fn configure(&mut self, config: Config) -> Result<bool, Error> {
        self.inner.configure(config)
    }

    fn kind() -> WatcherKind {
        RecommendedWatcher::kind()
    }
}
