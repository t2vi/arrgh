//! In-memory log ring buffer + tracing layer (ADR 0033, S1 #123). Port of
//! the .NET `LogService` + `RingBufferLoggerProvider`: a 500-entry buffer
//! with its own runtime-adjustable level gate, independent of the console's
//! static `LOG_LEVEL`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const CAPACITY: usize = 500;

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Shared ring buffer + level gate, read by `/api/logs` and appended to by
/// [`LogBufferLayer`].
pub struct LogBuffer {
    entries: Mutex<VecDeque<LogEntry>>,
    level: RwLock<String>,
}

impl LogBuffer {
    pub fn new(initial_level: &str) -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::new(VecDeque::with_capacity(CAPACITY)),
            level: RwLock::new(initial_level.to_uppercase()),
        })
    }

    fn append(&self, entry: LogEntry) {
        let mut buf = self.entries.lock().unwrap();
        if buf.len() >= CAPACITY {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    pub fn recent(&self, limit: usize) -> Vec<LogEntry> {
        let buf = self.entries.lock().unwrap();
        let limit = limit.clamp(1, CAPACITY);
        buf.iter().rev().take(limit).rev().cloned().collect()
    }

    pub fn current_level(&self) -> String {
        self.level.read().unwrap().clone()
    }

    /// `Some(())` on success, `None` if `level` isn't a recognised name.
    pub fn set_level(&self, level: &str) -> Option<()> {
        parse_level(level)?;
        *self.level.write().unwrap() = level.to_uppercase();
        Some(())
    }
}

fn parse_level(level: &str) -> Option<Level> {
    match level.to_uppercase().as_str() {
        "ERROR" => Some(Level::ERROR),
        "WARN" => Some(Level::WARN),
        "INFO" => Some(Level::INFO),
        "DEBUG" => Some(Level::DEBUG),
        _ => None,
    }
}

fn is_tracked(target: &str) -> bool {
    target.starts_with("arrgh_server") || target.starts_with("tower_http")
}

/// `tracing_subscriber::Layer` that appends matching events into a
/// [`LogBuffer`]. Deliberately never reports `Interest::never()` for any
/// callsite (the default `Layer` impl already does this) so the level gate
/// stays live — a `PATCH /api/logs/level` takes effect immediately, the
/// same way `RingBufferLogger.IsEnabled` re-checks on every log call.
pub struct LogBufferLayer {
    buffer: Arc<LogBuffer>,
}

impl LogBufferLayer {
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        Self { buffer }
    }
}

impl<S: Subscriber> Layer<S> for LogBufferLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let target = event.metadata().target();
        if !is_tracked(target) {
            return;
        }
        let gate = parse_level(&self.buffer.current_level()).unwrap_or(Level::INFO);
        if *event.metadata().level() > gate {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        self.buffer.append(LogEntry {
            timestamp: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
            level: event.metadata().level().to_string(),
            target: target.to_string(),
            message: visitor.message,
        });
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    #[test]
    fn evicts_oldest_beyond_capacity() {
        let buf = LogBuffer::new("info");
        for i in 0..CAPACITY + 10 {
            buf.append(LogEntry {
                timestamp: "t".into(),
                level: "INFO".into(),
                target: "arrgh_server".into(),
                message: i.to_string(),
            });
        }
        let recent = buf.recent(CAPACITY);
        assert_eq!(recent.len(), CAPACITY);
        assert_eq!(recent[0].message, "10");
        assert_eq!(recent.last().unwrap().message, (CAPACITY + 9).to_string());
    }

    #[test]
    fn set_level_rejects_unknown_value() {
        let buf = LogBuffer::new("info");
        assert!(buf.set_level("nonsense").is_none());
        assert_eq!(buf.current_level(), "INFO");
    }

    #[test]
    fn set_level_accepts_known_values_case_insensitive() {
        let buf = LogBuffer::new("info");
        assert!(buf.set_level("debug").is_some());
        assert_eq!(buf.current_level(), "DEBUG");
    }

    #[test]
    fn layer_captures_tracked_event_within_gate() {
        let buffer = LogBuffer::new("info");
        let layer = LogBufferLayer::new(buffer.clone());
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "arrgh_server::test", "hello");
            tracing::debug!(target: "arrgh_server::test", "too verbose");
            tracing::info!(target: "some_other_crate", "ignored target");
        });
        let entries = buffer.recent(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "hello");
    }
}
