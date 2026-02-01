use mds_app::logging::{LogField, LogLevel, Logger};

/// `tracing` adapter implementing the application logging port.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingLogger;

fn format_fields(fields: &[LogField]) -> String {
    if fields.is_empty() {
        return String::new();
    }
    fields
        .iter()
        .map(|f| format!("{}={}", f.key, f.value))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Logger for TracingLogger {
    fn log(&self, level: LogLevel, message: &str, fields: &[LogField]) {
        let fields_str = format_fields(fields);
        match level {
            LogLevel::Debug => tracing::debug!(fields = %fields_str, "{message}"),
            LogLevel::Info => tracing::info!(fields = %fields_str, "{message}"),
            LogLevel::Warn => tracing::warn!(fields = %fields_str, "{message}"),
            LogLevel::Error => tracing::error!(fields = %fields_str, "{message}"),
        }
    }
}
