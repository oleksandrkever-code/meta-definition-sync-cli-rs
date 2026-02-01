use clap::{Parser, Subcommand};

/// Root CLI entrypoint.
#[derive(Debug, Parser)]
#[command(name = "mdsr-cli")]
#[command(about = "Meta Definition Sync CLI (Rust rewrite)", long_about = None)]
pub struct Cli {
    /// Run in CI mode (no interactive prompts)
    #[arg(global = true)]
    #[arg(long, default_value_t = false)]
    pub ci: bool,

    /// Specify environment name (e.g. "my-dev" for `.env.my-dev`)
    #[arg(global = true)]
    #[arg(long)]
    pub environment: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print version info (and optionally check for updates).
    Version {
        /// Check GitHub Releases for a newer version.
        #[arg(long, default_value_t = false)]
        check: bool,
    },
    Metafield {
        #[command(subcommand)]
        command: MetafieldCommand,
    },
    Metaobject {
        #[command(subcommand)]
        command: MetaobjectCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum MetafieldCommand {
    /// Export metafield definitions into JSON files.
    Export {
        /// Owner type (e.g. PRODUCT)
        #[arg(long = "owner-type")]
        #[arg(default_value = "ALL")]
        owner_type: String,
    },
    /// Import metafield definitions from JSON files into Shopify.
    Import {
        /// Owner type (e.g. PRODUCT)
        #[arg(long = "owner-type")]
        owner_type: Option<String>,

        /// Allow recreation when a definition `type` differs.
        #[arg(long = "allow-type-changes")]
        #[arg(default_value_t = false)]
        allow_type_changes: bool,

        /// Allow deletion of associated metafields when recreating certain types.
        #[arg(long = "allow-associated-metafields-deletion")]
        #[arg(default_value_t = false)]
        allow_associated_metafields_deletion: bool,
    },
}
// metaobject
#[derive(Debug, Subcommand)]
pub enum MetaobjectCommand {
    /// Export metaobject definitions into JSON file.
    Export {},
    /// Import metaobject definitions from JSON file into Shopify.
    Import {},
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metafield_export_owner_type() {
        // Human explanation:
        // This test guarantees that the CLI accepts the command shape we want:
        //   mdsr-cli metafield export --owner-type PRODUCT
        // If we change flags/subcommands later, this test will fail and force us
        // to consciously update CLI behavior.
        let cli = Cli::try_parse_from([
            "mdsr-cli",
            "metafield",
            "export",
            "--owner-type",
            "PRODUCT",
        ])
        .unwrap();

        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Export { owner_type } => {
                    assert_eq!(owner_type, "PRODUCT");
                }
                MetafieldCommand::Import { .. } => panic!("expected metafield export"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metafield_export_owner_type_defaults_to_all() {
        // Human explanation:
        // For convenience (and parity with typical CLI UX), we want:
        //   mdsr-cli metafield export
        // to behave like:
        //   mdsr-cli metafield export --owner-type ALL
        let cli = Cli::try_parse_from(["mdsr-cli", "metafield", "export"]).unwrap();

        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Export { owner_type } => {
                    assert_eq!(owner_type, "ALL");
                }
                MetafieldCommand::Import { .. } => panic!("expected metafield export"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metafield_export_owner_type_all() {
        // Human explanation:
        // We want parity with the Node CLI behavior where `--owner-type ALL`
        // means "export for all owner types".
        let cli = Cli::try_parse_from(["mdsr-cli", "metafield", "export", "--owner-type", "ALL"])
            .unwrap();

        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Export { owner_type } => {
                    assert_eq!(owner_type, "ALL");
                }
                MetafieldCommand::Import { .. } => panic!("expected metafield export"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metafield_export_owner_type_list() {
        // Human explanation:
        // We support comma-separated owner types in a single flag value.
        let cli = Cli::try_parse_from([
            "mdsr-cli",
            "metafield",
            "export",
            "--owner-type",
            "PRODUCT,COLLECTION",
        ])
        .unwrap();

        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Export { owner_type } => {
                    assert_eq!(owner_type, "PRODUCT,COLLECTION");
                }
                MetafieldCommand::Import { .. } => panic!("expected metafield export"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metafield_import_product_defaults() {
        let cli = Cli::try_parse_from(["mdsr-cli", "metafield", "import"]).unwrap();
        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Import {
                    owner_type,
                    allow_type_changes,
                    allow_associated_metafields_deletion,
                } => {
                    assert_eq!(owner_type, None);
                    assert!(!allow_type_changes);
                    assert!(!allow_associated_metafields_deletion);
                }
                _ => panic!("expected metafield import"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metafield_import_owner_type() {
        let cli = Cli::try_parse_from([
            "mdsr-cli",
            "metafield",
            "import",
            "--owner-type",
            "PRODUCT,COLLECTION",
        ])
        .unwrap();
        match cli.command {
            Command::Metafield { command } => match command {
                MetafieldCommand::Import { owner_type, .. } => {
                    assert_eq!(owner_type.as_deref(), Some("PRODUCT,COLLECTION"));
                }
                _ => panic!("expected metafield import"),
            },
            Command::Metaobject { .. } => panic!("expected metafield command"),
            Command::Version { .. } => panic!("expected metafield command"),
        }
    }

    #[test]
    fn parses_metaobject_export() {
        let cli = Cli::try_parse_from(["mdsr-cli", "metaobject", "export"]).unwrap();
        match cli.command {
            Command::Metaobject { command } => match command {
                MetaobjectCommand::Export {} => {}
                MetaobjectCommand::Import {} => panic!("expected metaobject export"),
            },
            Command::Metafield { .. } => panic!("expected metaobject command"),
            Command::Version { .. } => panic!("expected metaobject command"),
        }
    }

    #[test]
    fn parses_metaobject_import() {
        let cli = Cli::try_parse_from(["mdsr-cli", "metaobject", "import"]).unwrap();
        match cli.command {
            Command::Metaobject { command } => match command {
                MetaobjectCommand::Import {} => {}
                MetaobjectCommand::Export {} => panic!("expected metaobject import"),
            },
            Command::Metafield { .. } => panic!("expected metaobject command"),
            Command::Version { .. } => panic!("expected metaobject command"),
        }
    }

    #[test]
    fn parses_global_ci_and_environment_flags() {
        // Human explanation:
        // We need global flags for env selection, same idea as the Node CLI:
        // - `--ci` disables interactive prompts
        // - `--environment <name>` selects `.env.<name>`
        let cli = Cli::try_parse_from([
            "mdsr-cli",
            "--ci",
            "--environment",
            "my-dev",
            "metafield",
            "export",
        ])
        .unwrap();

        assert!(cli.ci);
        assert_eq!(cli.environment.as_deref(), Some("my-dev"));
    }
}

