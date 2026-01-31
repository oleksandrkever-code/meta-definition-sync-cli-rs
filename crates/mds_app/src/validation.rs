//! Validation helpers (serde + serde_path_to_error) for Zod-like error reporting.

use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

pub fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}. {}: {}", i + 1, e.path, e.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse JSON into T while capturing the failing JSON path (Zod-like).
///
/// This does **structural** validation:
/// - wrong types
/// - missing required fields
/// - unknown fields (when using `#[serde(deny_unknown_fields)]`)
/// - invalid enum variants
pub fn parse_json_with_path<T: DeserializeOwned>(json: &str) -> Result<T, Vec<ValidationError>> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    match serde_path_to_error::deserialize::<_, T>(&mut deserializer) {
        Ok(v) => Ok(v),
        Err(e) => {
            let path = e.path().to_string();
            let message = e.inner().to_string();
            Err(vec![ValidationError { path, message }])
        }
    }
}

