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

fn main() {
    let cli = Cli::parse();

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

    init_logging(config.log_format);

    let base_logger = TracingLogger::default();
    let env_ctx = [LogField::new("env", config.env_name.clone())];
    let env_logger = ContextLogger::new(&base_logger, &env_ctx);

    match cli.command {
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
