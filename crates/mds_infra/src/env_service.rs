use std::{collections::HashMap, fs, path::PathBuf};

use mds_app::{
    config::{Environment, EnvironmentService, LogFormat, StoreConfig},
    AppError,
};

/// Environment/config loader using `.env` / `.env.<name>` files.
///
/// Reads files and returns a `StoreConfig` **without mutating** global `std::env`.
pub struct DotenvEnvironmentService {
    root: PathBuf,
}

impl DotenvEnvironmentService {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn detect_from_filenames(&self, filenames: &[String]) -> Vec<Environment> {
        let mut envs = vec![];
        for file_name in filenames {
            if file_name == ".env" {
                envs.push(Environment {
                    name: "default".to_string(),
                    display_name: "Default".to_string(),
                    file_path: self.root.join(file_name),
                });
            } else if let Some(rest) = file_name.strip_prefix(".env.") {
                if rest.is_empty() {
                    continue;
                }
                let mut chars = rest.chars();
                let display = match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => rest.to_string(),
                };
                envs.push(Environment {
                    name: rest.to_string(),
                    display_name: display,
                    file_path: self.root.join(file_name),
                });
            }
        }
        // Sort: default first, then by display name.
        envs.sort_by(|a, b| {
            if a.name == "default" && b.name != "default" {
                std::cmp::Ordering::Less
            } else if b.name == "default" && a.name != "default" {
                std::cmp::Ordering::Greater
            } else {
                a.display_name.cmp(&b.display_name)
            }
        });
        envs
    }

    fn parse_env_file(&self, path: &PathBuf) -> Result<HashMap<String, String>, AppError> {
        let iter = dotenvy::from_path_iter(path).map_err(|e| {
            AppError::Config(format!("failed to read env file {}: {}", path.display(), e))
        })?;

        let mut map = HashMap::new();
        for item in iter {
            let (k, v) = item.map_err(|e| {
                AppError::Config(format!(
                    "failed to parse env file {}: {}",
                    path.display(),
                    e
                ))
            })?;
            map.insert(k, v);
        }
        Ok(map)
    }

    fn required(map: &HashMap<String, String>, key: &str) -> Result<String, AppError> {
        map.get(key)
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AppError::Config(format!("missing required env var: {key}")))
    }

    fn parse_bool(map: &HashMap<String, String>, key: &str) -> bool {
        match map.get(key).map(|s| s.trim().to_ascii_lowercase()) {
            Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
            _ => false,
        }
    }

    fn parse_u64(map: &HashMap<String, String>, key: &str) -> Option<u64> {
        map.get(key).and_then(|s| s.trim().parse::<u64>().ok())
    }
}

impl EnvironmentService for DotenvEnvironmentService {
    fn detect(&self) -> Result<Vec<Environment>, AppError> {
        let entries = fs::read_dir(&self.root).map_err(|e| {
            AppError::Config(format!(
                "cannot read directory {}: {}",
                self.root.display(),
                e
            ))
        })?;
        let filenames: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        let envs = self.detect_from_filenames(&filenames);
        if envs.is_empty() {
            return Err(AppError::Config(
                "no environment files found (.env or .env.<name>)".into(),
            ));
        }
        Ok(envs)
    }

    fn load_store_config(&self, env: &Environment) -> Result<StoreConfig, AppError> {
        // Start with real process env (allows `export FOO=...`), then let `.env` override it.
        // Still: do not mutate global env.
        let mut map: HashMap<String, String> = std::env::vars().collect();
        let file_map = self.parse_env_file(&env.file_path)?;
        for (k, v) in file_map {
            map.insert(k, v);
        }

        let shop_domain = Self::required(&map, "MDS_CLI_SHOPIFY_SHOP_DOMAIN")?;
        let access_token = Self::required(&map, "MDS_CLI_SHOPIFY_ACCESS_TOKEN")?;
        let log_format = map
            .get("MDS_LOG_FORMAT")
            .map(|s| LogFormat::from_str(s))
            .unwrap_or(LogFormat::Pretty);

        Ok(StoreConfig {
            env_name: env.name.clone(),
            shop_domain,
            access_token,
            log_format,
            disable_update_check: Self::parse_bool(&map, "MDSR_CLI_NO_UPDATE_CHECK"),
            update_check_days: Self::parse_u64(&map, "MDSR_CLI_UPDATE_CHECK_DAYS").unwrap_or(7),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_env_files_from_names() {
        // Human explanation:
        // Same detection behavior as Node: `.env` + `.env.<name>`.
        // We avoid creating real `.env*` files in tests because the parent repo `.gitignore`
        // ignores them, and the sandbox blocks writes to ignored paths.
        let svc = DotenvEnvironmentService::new(PathBuf::from("/project/root"));
        let envs = svc.detect_from_filenames(&vec![
            ".env".to_string(),
            ".env.my-dev".to_string(),
            "README.md".to_string(),
        ]);
        assert_eq!(envs.len(), 2);
        assert_eq!(envs[0].name, "default");
        assert_eq!(envs[1].name, "my-dev");
    }
}
