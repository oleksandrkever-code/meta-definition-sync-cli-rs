//! Infrastructure layer (adapters: filesystem, HTTP clients, env loading, etc.)

pub mod env_service;
pub mod shopify;
pub mod logger;

pub use shopify::gateway::ShopifyMetafieldGateway;

use std::{fs, path::Path};

use mds_app::{AppError, FileRepo};

pub struct FsFileRepo;

impl FsFileRepo {
    pub fn new() -> Self {
        Self
    }
}

impl FileRepo for FsFileRepo {
    fn read_text(&self, path: &str) -> Result<String, AppError> {
        let p = Path::new(path);
        fs::read_to_string(p).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::Repo(format!("file not found: {path}"))
            } else {
                AppError::Repo(e.to_string())
            }
        })
    }

    fn write_text(&mut self, path: &str, contents: &str) -> Result<(), AppError> {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::Repo(e.to_string()))?;
        }
        fs::write(p, contents).map_err(|e| AppError::Repo(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn fs_file_repo_writes_file_and_creates_parent_dirs() {
        // Human explanation:
        // Our app-layer use-case writes to `definitions/metafields/product.json`.
        // The infrastructure file repo must be able to:
        // - create missing parent directories
        // - write the file contents
        // This test locks that behavior down without any Shopify involved.

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let base = std::env::temp_dir().join(format!("mds_cli_test_{nanos}"));
        let target = base.join("../../../definitions1/metafields/product.json");

        let mut repo = FsFileRepo::new();
        repo.write_text(target.to_str().unwrap(), "[{\"namespace\":\"custom\",\"key\":\"k\"}]")
            .unwrap();

        let written = fs::read_to_string(&target).unwrap();
        assert!(written.contains("\"namespace\""));

        // cleanup best-effort
        let _ = fs::remove_dir_all(base);
    }
}
