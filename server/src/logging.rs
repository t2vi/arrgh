use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};
use chrono::Utc;
use serde::Serialize;
use tracing::Subscriber;
use tracing_subscriber::{layer::Context, Layer};

pub const LEVEL_ERROR: u8 = 1;
pub const LEVEL_WARN:  u8 = 2;
pub const LEVEL_INFO:  u8 = 3;
pub const LEVEL_DEBUG: u8 = 4;

const BUFFER_CAPACITY: usize = 500;

#[derive(Serialize, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;
pub type LevelGate = Arc<AtomicU8>;

pub fn new_buffer() -> LogBuffer {
    Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY)))
}

pub fn level_from_str(s: &str) -> Option<u8> {
    match s.to_uppercase().as_str() {
        "ERROR" => Some(LEVEL_ERROR),
        "WARN"  => Some(LEVEL_WARN),
        "INFO"  => Some(LEVEL_INFO),
        "DEBUG" => Some(LEVEL_DEBUG),
        _       => None,
    }
}

pub fn level_to_str(v: u8) -> &'static str {
    match v {
        LEVEL_ERROR => "ERROR",
        LEVEL_WARN  => "WARN",
        LEVEL_DEBUG => "DEBUG",
        _           => "INFO",
    }
}

pub struct RingBufferLayer {
    buffer: LogBuffer,
    gate: LevelGate,
}

impl RingBufferLayer {
    pub fn new(buffer: LogBuffer, gate: LevelGate) -> Self {
        Self { buffer, gate }
    }
}

impl<S: Subscriber> Layer<S> for RingBufferLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let target = meta.target();

        // Only capture arrgh_server events
        if !target.starts_with("arrgh_server") {
            return;
        }

        let event_level: u8 = match *meta.level() {
            tracing::Level::ERROR => LEVEL_ERROR,
            tracing::Level::WARN  => LEVEL_WARN,
            tracing::Level::INFO  => LEVEL_INFO,
            tracing::Level::DEBUG | tracing::Level::TRACE => LEVEL_DEBUG,
        };

        if event_level > self.gate.load(Ordering::Relaxed) {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: meta.level().to_string(),
            target: target.to_string(),
            message: visitor.message,
        };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_roundtrip() {
        for (s, v) in [("ERROR", LEVEL_ERROR), ("WARN", LEVEL_WARN), ("INFO", LEVEL_INFO), ("DEBUG", LEVEL_DEBUG)] {
            assert_eq!(level_from_str(s), Some(v));
            assert_eq!(level_to_str(v), s);
        }
    }

    #[test]
    fn level_from_str_case_insensitive() {
        assert_eq!(level_from_str("error"), Some(LEVEL_ERROR));
        assert_eq!(level_from_str("warn"),  Some(LEVEL_WARN));
        assert_eq!(level_from_str("info"),  Some(LEVEL_INFO));
        assert_eq!(level_from_str("debug"), Some(LEVEL_DEBUG));
    }

    #[test]
    fn level_from_str_unknown_returns_none() {
        assert_eq!(level_from_str("trace"), None);
        assert_eq!(level_from_str(""),      None);
        assert_eq!(level_from_str("FATAL"), None);
    }

    #[test]
    fn level_to_str_unknown_defaults_info() {
        assert_eq!(level_to_str(99), "INFO");
        assert_eq!(level_to_str(0),  "INFO");
    }

    #[test]
    fn ring_buffer_evicts_oldest_at_capacity() {
        let buf = new_buffer();
        {
            let mut b = buf.lock().unwrap();
            for i in 0..BUFFER_CAPACITY + 1 {
                if b.len() >= BUFFER_CAPACITY {
                    b.pop_front();
                }
                b.push_back(LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: "INFO".into(),
                    target: "test".into(),
                    message: format!("msg {i}"),
                });
            }
            assert_eq!(b.len(), BUFFER_CAPACITY);
            assert_eq!(b.front().unwrap().message, "msg 1");
        }
    }
}
