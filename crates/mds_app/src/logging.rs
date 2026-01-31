//! Logging port (Clean Architecture).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogField {
    pub key: &'static str,
    pub value: String,
}

impl LogField {
    pub fn new(key: &'static str, value: impl Into<String>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }
}

/// Application-level logging port.
///
/// - No dependency on `tracing`, `log`, etc.
/// - Uses runtime fields (key/value strings) for portability.
pub trait Logger {
    fn log(&self, level: LogLevel, message: &str, fields: &[LogField]);
}

/// Adds constant context fields to any `Logger`.
#[derive(Clone, Copy)]
pub struct ContextLogger<'a> {
    inner: &'a dyn Logger,
    ctx: &'a [LogField],
}

impl<'a> ContextLogger<'a> {
    pub fn new(inner: &'a dyn Logger, ctx: &'a [LogField]) -> Self {
        Self { inner, ctx }
    }

    pub fn with<'b>(&'b self, more: &'b [LogField]) -> ContextLogger<'b> {
        // Create a view that uses caller-provided slice as context overlay.
        // (Caller can pass combined slices if they want deeper nesting.)
        ContextLogger { inner: self, ctx: more }
    }
}

impl Logger for ContextLogger<'_> {
    fn log(&self, level: LogLevel, message: &str, fields: &[LogField]) {
        // Merge: ctx fields first, then call-specific fields.
        // We avoid allocations by logging twice: first ctx then fields
        // in one combined vector would allocate; instead we pass combined slice via a small stack vec.
        let mut merged: Vec<LogField> = Vec::with_capacity(self.ctx.len() + fields.len());
        merged.extend_from_slice(self.ctx);
        merged.extend_from_slice(fields);
        self.inner.log(level, message, &merged);
    }
}

