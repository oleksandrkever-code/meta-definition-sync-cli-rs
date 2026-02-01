//! Application-layer error type.

#[derive(Debug)]
pub enum AppError {
    Gateway(String),
    Repo(String),
    Json(String),
    Config(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Gateway(msg) => write!(f, "gateway error: {msg}"),
            AppError::Repo(msg) => write!(f, "repo error: {msg}"),
            AppError::Json(msg) => write!(f, "json error: {msg}"),
            AppError::Config(msg) => write!(f, "config error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}
