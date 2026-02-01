mod cli;

use clap::Parser;
use cli::{Cli, Command, MetafieldCommand, MetaobjectCommand};
use dialoguer::Select;
use mds_app::{
    ExportMetafieldsToFileUseCase, ExportMetaobjectsToFileUseCase, ImportMetafieldsFromFileUseCase,
    ImportMetafieldsOptions, ImportMetaobjectsFromFileUseCase, PlanMetaobjectsImportUseCase,
    SystemClock,
};
use mds_app::config::{Environment, EnvironmentService, LogFormat};
use mds_app::logging::{ContextLogger, LogField, LogLevel, Logger};
use mds_domain::{parse_owner_types, OwnerType};
use mds_infra::{env_service::DotenvEnvironmentService, ShopifyMetafieldGateway, FsFileRepo};
use mds_infra::logger::TracingLogger;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::time::Duration;

fn main() {
    let cli = Cli::parse();

    // Commands that should work without any Shopify env/config:
    if let Command::Version { check } = cli.command {
        print_version(check);
        return;
    }

    // Load config from `.env` / `.env.<name>` WITHOUT mutating global env.
    // This makes future `diff` between two stores possible.
    let service = DotenvEnvironmentService::new(std::env::current_dir().unwrap());
    let envs = match service.detect() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let selected_env = match select_environment(&cli, &envs) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let config = match service.load_store_config(selected_env) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    maybe_check_for_updates_on_startup(&cli, &config);

    init_logging(config.log_format);

    let base_logger = TracingLogger::default();
    let env_ctx = [LogField::new("env", config.env_name.clone())];
    let env_logger = ContextLogger::new(&base_logger, &env_ctx);

    match cli.command {
        Command::Version { .. } => unreachable!("handled before env loading"),
        Command::Metafield { command } => match command {
            MetafieldCommand::Export { owner_type } => {
                let run_ctx = [LogField::new("command", "metafield")];
                let logger = ContextLogger::new(&env_logger, &run_ctx);
                let gateway = ShopifyMetafieldGateway::new(&config);
                let usecase = ExportMetafieldsToFileUseCase::new(gateway);
                let mut repo = FsFileRepo::new();

                let owner_types = match parse_owner_types(&owner_type) {
                    Ok(v) if !v.is_empty() => v,
                    Ok(_) => {
                        logger.log(
                            LogLevel::Error,
                            "No owner types provided in --owner-type",
                            &[LogField::new("owner_type", owner_type)],
                        );
                        std::process::exit(2);
                    }
                    Err(e) => {
                        logger.log(
                            LogLevel::Error,
                            "Invalid --owner-type value",
                            &[
                                LogField::new("owner_type", owner_type),
                                LogField::new("error", e.to_string()),
                            ],
                        );
                        std::process::exit(2);
                    }
                };

                let mut any_failed = false;
                for ot in owner_types {
                    let ot_ctx = [LogField::new("owner_type", ot.as_str())];
                    let ot_logger = ContextLogger::new(&logger, &ot_ctx);

                    match usecase.execute(ot, &mut repo, &ot_logger) {
                        Ok(defs) => {
                            let owner = ot.as_str().to_ascii_lowercase();
                            ot_logger.log(
                                LogLevel::Info,
                                "Export completed",
                                &[
                                    LogField::new("exported_count", defs.len().to_string()),
                                    LogField::new(
                                        "output",
                                        format!("definitions/metafields/{}.json", owner),
                                    ),
                                ],
                            );
                        }
                        Err(e) => {
                            any_failed = true;
                            ot_logger.log(
                                LogLevel::Error,
                                "Export failed",
                                &[LogField::new("error", e.to_string())],
                            );
                        }
                    }
                }

                if any_failed {
                    std::process::exit(1);
                }
            }
            MetafieldCommand::Import {
                owner_type,
                allow_type_changes,
                allow_associated_metafields_deletion,
            } => {
                let run_ctx = [LogField::new("command", "metafield")];
                let logger = ContextLogger::new(&env_logger, &run_ctx);
                let gateway = ShopifyMetafieldGateway::new(&config);
                let clock = SystemClock;
                let usecase = ImportMetafieldsFromFileUseCase::new(gateway, clock);
                let mut repo = FsFileRepo::new();

                // Owner type selection:
                // - CI: require explicit --owner-type
                // - non-CI: prompt if not provided
                let selected_owner_type = match owner_type.as_deref() {
                    Some(v) => v.to_string(),
                    None => {
                        if cli.ci {
                            logger.log(
                                LogLevel::Error,
                                "--owner-type is required in CI mode",
                                &[],
                            );
                            std::process::exit(2);
                        }

                        let mut items = vec!["ALL".to_string()];
                        items.extend(OwnerType::all().into_iter().map(|ot| ot.as_str().to_string()));
                        let selection = Select::new()
                            .with_prompt("Select metafield owner type to import")
                            .items(&items)
                            .default(0)
                            .interact()
                            .map_err(|e| e.to_string());
                        match selection {
                            Ok(idx) => items[idx].clone(),
                            Err(e) => {
                                logger.log(
                                    LogLevel::Error,
                                    "Failed to read selection",
                                    &[LogField::new("error", e)],
                                );
                                std::process::exit(2);
                            }
                        }
                    }
                };

                let selection_is_all = selected_owner_type.trim().eq_ignore_ascii_case("ALL");

                let owner_types = match parse_owner_types(&selected_owner_type) {
                    Ok(v) if !v.is_empty() => v,
                    Ok(_) => {
                        logger.log(
                            LogLevel::Error,
                            "No owner types provided in --owner-type",
                            &[LogField::new("owner_type", selected_owner_type)],
                        );
                        std::process::exit(2);
                    }
                    Err(e) => {
                        logger.log(
                            LogLevel::Error,
                            "Invalid --owner-type value",
                            &[
                                LogField::new("owner_type", selected_owner_type),
                                LogField::new("error", e.to_string()),
                            ],
                        );
                        std::process::exit(2);
                    }
                };

                let options = ImportMetafieldsOptions {
                    allow_type_changes,
                    allow_associated_metafields_deletion,
                };

                let is_multi_owner = owner_types.len() > 1;

                let mut any_failed = false;
                for ot in owner_types {
                    let ot_ctx = [LogField::new("owner_type", ot.as_str())];
                    let ot_logger = ContextLogger::new(&logger, &ot_ctx);

                    if !cli.ci && !selection_is_all {
                        // Parity with as-is: confirmation prompt per owner type when selection is not ALL.
                        let proceed = dialoguer::Confirm::new()
                            .with_prompt("Proceed?")
                            .default(false)
                            .interact()
                            .unwrap_or(false);
                        if !proceed {
                            ot_logger.log(LogLevel::Info, "Skipped by user", &[]);
                            continue;
                        }
                    }

                    match usecase.execute(ot, &mut repo, options, &ot_logger) {
                        Ok(report) => {
                            if report.summary.failed > 0 {
                                any_failed = true;
                            }
                            ot_logger.log(
                                LogLevel::Info,
                                "Import completed",
                                &[
                                    LogField::new("created", report.summary.created.to_string()),
                                    LogField::new("updated", report.summary.updated.to_string()),
                                    LogField::new("recreated", report.summary.recreated.to_string()),
                                    LogField::new("noChange", report.summary.no_change.to_string()),
                                    LogField::new("failed", report.summary.failed.to_string()),
                                    LogField::new("total", report.summary.total.to_string()),
                                ],
                            );
                        }
                        Err(e) => {
                            let msg = e.to_string();

                            // Parity with as-is when importing multiple/all owner types:
                            // missing file -> warn and skip that owner type.
                            if is_multi_owner && msg.contains("file not found:") {
                                ot_logger.log(
                                    LogLevel::Warn,
                                    "Input file not found; skipping owner type",
                                    &[LogField::new("error", msg)],
                                );
                                continue;
                            }

                            any_failed = true;
                            ot_logger.log(
                                LogLevel::Error,
                                "Import failed",
                                &[LogField::new("error", msg)],
                            );
                        }
                    }
                }

                if any_failed {
                    std::process::exit(1);
                }
            }
        },
        Command::Metaobject { command } => match command {
            MetaobjectCommand::Export {} => {
                let run_ctx = [LogField::new("command", "metaobject")];
                let logger = ContextLogger::new(&env_logger, &run_ctx);

                let gateway = ShopifyMetafieldGateway::new(&config);
                let usecase = ExportMetaobjectsToFileUseCase::new(gateway);
                let mut repo = FsFileRepo::new();

                match usecase.execute(&mut repo, &logger) {
                    Ok(defs) => {
                        logger.log(
                            LogLevel::Info,
                            "Export completed",
                            &[
                                LogField::new("exported_count", defs.len().to_string()),
                                LogField::new("output", "definitions/metaobjects.json"),
                            ],
                        );
                    }
                    Err(e) => {
                        logger.log(
                            LogLevel::Error,
                            "Export failed",
                            &[LogField::new("error", e.to_string())],
                        );
                        std::process::exit(1);
                    }
                }
            }
            MetaobjectCommand::Import {} => {
                let run_ctx = [LogField::new("command", "metaobject")];
                let logger = ContextLogger::new(&env_logger, &run_ctx);

                let gateway = ShopifyMetafieldGateway::new(&config);
                let clock = SystemClock;
                let planner = PlanMetaobjectsImportUseCase::new(gateway.clone());
                let importer = ImportMetaobjectsFromFileUseCase::new(gateway, clock);
                let mut repo = FsFileRepo::new();

                let plan = match planner.execute(&mut repo, &logger) {
                    Ok(p) => p,
                    Err(e) => {
                        logger.log(
                            LogLevel::Error,
                            "Import planning failed",
                            &[LogField::new("error", e.to_string())],
                        );
                        std::process::exit(2);
                    }
                };

                // Print dependency tree (Markdown, directory-like) before running mutations.
                println!("{}", plan.tree_markdown);

                if !cli.ci {
                    let proceed = dialoguer::Confirm::new()
                        .with_prompt("Proceed?")
                        .default(false)
                        .interact()
                        .unwrap_or(false);
                    if !proceed {
                        logger.log(LogLevel::Info, "Skipped by user", &[]);
                        return;
                    }
                }

                match importer.execute(&plan, &mut repo, &logger) {
                    Ok(report) => {
                        logger.log(
                            LogLevel::Info,
                            "Import completed",
                            &[
                                LogField::new("created", report.summary.created.to_string()),
                                LogField::new("updated", report.summary.updated.to_string()),
                                LogField::new("noChange", report.summary.no_change.to_string()),
                                LogField::new("failed", report.summary.failed.to_string()),
                                LogField::new("total", report.summary.total.to_string()),
                            ],
                        );
                        if report.summary.failed > 0 {
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        logger.log(
                            LogLevel::Error,
                            "Import failed",
                            &[LogField::new("error", e.to_string())],
                        );
                        std::process::exit(1);
                    }
                }
            }
        },
    }
}

fn maybe_check_for_updates_on_startup(cli: &Cli, config: &mds_app::config::StoreConfig) {
    // Don't spam CI/logs.
    if cli.ci {
        return;
    }

    if config.disable_update_check {
        return;
    }

    // Only show update hints in interactive terminals.
    if !std::io::stderr().is_terminal() {
        return;
    }

    // Avoid double output for `version` (it has an explicit check mode).
    if matches!(cli.command, Command::Version { .. }) {
        return;
    }

    let now = unix_now_secs();
    let interval_days = config.update_check_days;
    let interval_secs = interval_days.saturating_mul(24 * 60 * 60);

    let cache_path = update_check_cache_path();
    if let Some(cache) = read_update_cache(&cache_path) {
        if now.saturating_sub(cache.last_checked_unix) < interval_secs {
            return;
        }
    }

    // Best-effort: update cache even if request fails (avoid retry loops).
    write_update_cache(&cache_path, &UpdateCheckCache { last_checked_unix: now });

    // Silent unless update exists.
    if let Some(msg) = check_update_message(Duration::from_secs(2)) {
        eprintln!("{msg}");
    }
}

fn print_version(check: bool) {
    let current = format!("v{}", env!("CARGO_PKG_VERSION"));
    println!("{current}");

    if !check {
        return;
    }

    match check_update_message(Duration::from_secs(4)) {
        Some(msg) => eprintln!("{msg}"),
        None => eprintln!("You are up to date ({current})."),
    };
}

fn check_update_message(timeout: Duration) -> Option<String> {
    let current = format!("v{}", env!("CARGO_PKG_VERSION"));
    let repo = "oleksandrkever-code/meta-definition-sync-cli-rs";
    let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let readme_install_url =
        "https://github.com/oleksandrkever-code/meta-definition-sync-cli-rs#install-no-rust-required";

    #[derive(Debug, Deserialize)]
    struct GithubRelease {
        tag_name: String,
        html_url: Option<String>,
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .ok()?;

    let resp = client
        .get(api_url)
        .header("User-Agent", format!("mdsr-cli/{}", env!("CARGO_PKG_VERSION")))
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let latest: GithubRelease = resp.json().ok()?;
    let latest_tag = latest.tag_name.trim().to_string();
    if latest_tag.is_empty() {
        return None;
    }

    match compare_semver_tags(&current, &latest_tag) {
        Some(std::cmp::Ordering::Less) => {
            let release_url = latest
                .html_url
                .as_deref()
                .unwrap_or("https://github.com/oleksandrkever-code/meta-definition-sync-cli-rs/releases/latest");
            Some(format!(
                "Update available: {current} -> {latest_tag}. See: {readme_install_url} (release: {release_url})"
            ))
        }
        _ => None,
    }
}

fn parse_v_semver(tag: &str) -> Option<(u64, u64, u64)> {
    let t = tag.trim();
    let t = t.strip_prefix('v').unwrap_or(t);
    let mut it = t.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next()?.parse().ok()?;
    Some((major, minor, patch))
}

fn compare_semver_tags(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let aa = parse_v_semver(a)?;
    let bb = parse_v_semver(b)?;
    Some(aa.cmp(&bb))
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateCheckCache {
    last_checked_unix: u64,
}

fn update_check_cache_path() -> std::path::PathBuf {
    // Linux: $XDG_CACHE_HOME or ~/.cache
    // macOS: ~/Library/Caches
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    #[cfg(target_os = "macos")]
    let base = std::path::PathBuf::from(home).join("Library").join("Caches");

    #[cfg(not(target_os = "macos"))]
    let base = std::env::var("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(home).join(".cache"));

    base.join("mdsr-cli").join("update-check.json")
}

fn read_update_cache(path: &std::path::Path) -> Option<UpdateCheckCache> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_update_cache(path: &std::path::Path, cache: &UpdateCheckCache) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec(cache) {
        let _ = std::fs::write(path, bytes);
    }
}

fn select_environment<'a>(cli: &Cli, envs: &'a [Environment]) -> Result<&'a Environment, String> {
    // 1) If explicitly requested, validate and return.
    if let Some(name) = cli.environment.as_deref() {
        return envs
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("Invalid environment \"{name}\"")) ;
    }

    // 2) If `.env` exists, prefer it (Node-like default environment behavior).
    if let Some(default) = envs.iter().find(|e| e.name == "default") {
        return Ok(default);
    }

    // 3) If only one named env exists, auto-pick it.
    if envs.len() == 1 {
        return Ok(&envs[0]);
    }

    // 4) If CI mode and multiple envs exist, require explicit selection.
    if cli.ci {
        let names = envs.iter().map(|e| e.name.clone()).collect::<Vec<_>>().join(", ");
        return Err(format!("--environment is required in CI mode. Available: {names}"));
    }

    // 5) Interactive selection.
    let items: Vec<String> = envs.iter().map(|e| e.display_name.clone()).collect();
    let selection = Select::new()
        .with_prompt("Multiple environments detected. Select one")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| e.to_string())?;
    Ok(&envs[selection])
}

fn init_logging(format: LogFormat) {
    // Logging format:
    // - pretty (default): human-friendly
    // - json: structured logs
    // Respect RUST_LOG if user sets it; otherwise default to info.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        // JSON logs: best for CI/log aggregation.
        LogFormat::Json => builder.json().init(),

        // NOTE:
        // `pretty()` formatter in tracing-subscriber includes source-location lines like:
        //   at crates/...:line
        // Even when file/line are disabled.
        // We prefer a clean, human-readable output without that extra line, so we use `compact()`
        // as the default "pretty" format.
        LogFormat::Pretty => builder.compact().init(),
    }
}
