//! Metafield import (application-layer orchestration + rules).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use mds_domain::OwnerType;

use crate::error::AppError;
use crate::logging::{LogField, LogLevel, Logger};
use crate::ports::{Clock, FileRepo, MetafieldImportGateway};

use super::types::{
    CapabilityFlag, MetafieldCapabilities, MetafieldDefinitionCapabilitiesInput,
    MetafieldDefinitionInput, MetafieldDefinitionValidationInput, MetafieldValidation,
    ShopifyMetafieldDefinition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportMetafieldsOptions {
    pub allow_type_changes: bool,
    pub allow_associated_metafields_deletion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetafieldImportAction {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "recreate")]
    Recreate,
    #[serde(rename = "no_change")]
    NoChange,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldImportItemReport {
    pub namespace: String,
    pub key: String,
    pub action: MetafieldImportAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MetafieldImportSummary {
    pub created: usize,
    pub updated: usize,
    pub recreated: usize,
    #[serde(rename = "noChange")]
    pub no_change: usize,
    pub failed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldImportReport {
    #[serde(rename = "ownerType")]
    pub owner_type: String,
    pub summary: MetafieldImportSummary,
    pub items: Vec<MetafieldImportItemReport>,
}

fn metafield_key(namespace: &str, key: &str) -> String {
    format!("{namespace}.{key}")
}

fn normalize_description(d: Option<String>) -> Option<String> {
    let s = d?.trim().to_string();
    if s.is_empty() {
        return None;
    }
    // Parity with as-is spec: truncate to 255.
    if s.len() > 255 {
        Some(s.chars().take(255).collect())
    } else {
        Some(s)
    }
}

fn normalize_validations(
    v: Option<Vec<crate::metafield_config::ValidationRule>>,
) -> Option<Vec<crate::metafield_config::ValidationRule>> {
    match v {
        None => None,
        Some(list) if list.is_empty() => None,
        Some(list) => Some(list),
    }
}

fn validations_to_input(
    validations: Option<Vec<crate::metafield_config::ValidationRule>>,
    metaobject_type_to_id: &HashMap<String, String>,
) -> Option<Vec<MetafieldDefinitionValidationInput>> {
    let v = normalize_validations(validations)?;
    let out = v
        .into_iter()
        .map(|r| {
            // Parity with Node importer: resolve metaobject_definition_type -> metaobject_definition_id.
            if r.name == "metaobject_definition_type" {
                if let Some(t) = r.value.as_deref() {
                    if let Some(id) = metaobject_type_to_id.get(t) {
                        return MetafieldDefinitionValidationInput {
                            name: "metaobject_definition_id".to_string(),
                            value: Some(id.clone()),
                        };
                    }
                }
            }
            MetafieldDefinitionValidationInput {
                name: r.name,
                value: r.value,
            }
        })
        .collect::<Vec<_>>();

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn capabilities_to_input(
    c: Option<crate::metafield_config::MetafieldCapabilitiesConfig>,
) -> Option<MetafieldDefinitionCapabilitiesInput> {
    c.map(|c| MetafieldDefinitionCapabilitiesInput {
        admin_filterable: c
            .admin_filterable
            .map(|x| CapabilityFlag { enabled: x.enabled }),
        smart_collection_condition: c
            .smart_collection_condition
            .map(|x| CapabilityFlag { enabled: x.enabled }),
        unique_values: c
            .unique_values
            .map(|x| CapabilityFlag { enabled: x.enabled }),
    })
}

fn are_validations_equal(
    a: &Option<Vec<MetafieldDefinitionValidationInput>>,
    b: &Vec<MetafieldValidation>,
) -> bool {
    // Parity with Node: deep-compare by JSON stringification.
    let a_json = match a {
        None => None,
        Some(v) if v.is_empty() => None,
        Some(v) => Some(serde_json::to_string(v).unwrap_or_default()),
    };
    let b_json = if b.is_empty() {
        None
    } else {
        Some(serde_json::to_string(b).unwrap_or_default())
    };
    a_json == b_json
}

fn are_capabilities_equal(
    cfg: &Option<crate::metafield_config::MetafieldCapabilitiesConfig>,
    existing: &Option<MetafieldCapabilities>,
) -> bool {
    match cfg {
        None => true, // "compare only if JSON explicitly contains capabilities"
        Some(c) => {
            let existing = existing.as_ref();

            let eq_flag = |cfg: &Option<crate::metafield_config::CapabilityFlagConfig>,
                           ex: Option<&CapabilityFlag>| {
                match (cfg, ex) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.enabled == b.enabled,
                    // if cfg mentions a flag but Shopify doesn't have it (or vice versa), treat as diff
                    _ => false,
                }
            };

            eq_flag(
                &c.admin_filterable,
                existing.and_then(|e| e.admin_filterable.as_ref()),
            ) && eq_flag(
                &c.smart_collection_condition,
                existing.and_then(|e| e.smart_collection_condition.as_ref()),
            ) && eq_flag(
                &c.unique_values,
                existing.and_then(|e| e.unique_values.as_ref()),
            )
        }
    }
}

fn should_update(
    cfg: &crate::metafield_config::MetafieldDefinitionConfig,
    input_validations: &Option<Vec<MetafieldDefinitionValidationInput>>,
    existing: &ShopifyMetafieldDefinition,
) -> bool {
    // name
    if existing.name != cfg.name {
        return true;
    }

    // description (normalized)
    let cfg_desc = normalize_description(cfg.description.clone());
    let ex_desc = normalize_description(existing.description.clone());
    if cfg_desc != ex_desc {
        return true;
    }

    // pin: cfg.pin vs existing.pinned_position.is_some()
    if existing.pinned_position.is_some() != cfg.pin {
        return true;
    }

    // validations
    if !are_validations_equal(input_validations, &existing.validations) {
        return true;
    }

    // capabilities only if explicitly present in JSON
    if !are_capabilities_equal(&cfg.capabilities, &existing.capabilities) {
        return true;
    }

    // access compare intentionally omitted for now:
    // - Node compares access only if JSON contains it
    // - but Node also strips access from mutation payloads, which makes access updates unreliable
    // We keep it out of change detection in the first vertical slice to avoid churn.

    false
}

pub struct ImportMetafieldsFromFileUseCase<G, C> {
    gateway: G,
    clock: C,
}

impl<G, C> ImportMetafieldsFromFileUseCase<G, C>
where
    G: MetafieldImportGateway,
    C: Clock,
{
    pub fn new(gateway: G, clock: C) -> Self {
        Self { gateway, clock }
    }

    pub fn execute(
        &self,
        owner_type: OwnerType,
        repo: &mut impl FileRepo,
        options: ImportMetafieldsOptions,
        logger: &dyn Logger,
    ) -> Result<MetafieldImportReport, AppError> {
        let owner = owner_type.as_str().to_ascii_lowercase();
        let input_path = format!("definitions/metafields/{owner}.json");

        let json = match repo.read_text(&input_path) {
            Ok(v) => v,
            Err(AppError::Repo(msg)) if msg.contains("file not found:") => {
                logger.log(
                    LogLevel::Info,
                    "Hint: run command to generate missing definitions file",
                    &[
                        LogField::new(
                            "suggested_command",
                            format!(
                                "mdsr-cli metafield export --owner-type {}",
                                owner_type.as_str()
                            ),
                        ),
                        LogField::new("output", input_path.clone()),
                    ],
                );
                return Err(AppError::Repo(msg));
            }
            Err(e) => return Err(e),
        };

        let parsed = crate::validation::parse_json_with_path::<
            Vec<crate::metafield_config::MetafieldDefinitionConfig>,
        >(&json)
        .map_err(|errs| AppError::Json(crate::validation::format_validation_errors(&errs)))?;

        if parsed.is_empty() {
            return Err(AppError::Json(format!(
                "metafield config file must be a non-empty array: {input_path}"
            )));
        }

        let existing = self
            .gateway
            .list_existing_metafield_definitions(owner_type, logger)?;
        let mut existing_map: HashMap<String, ShopifyMetafieldDefinition> = HashMap::new();
        for d in existing {
            existing_map.insert(metafield_key(&d.namespace, &d.key), d);
        }

        let metaobject_type_to_id = self.gateway.metaobject_type_to_id_map(logger)?;

        let mut report = MetafieldImportReport {
            owner_type: owner_type.as_str().to_string(),
            summary: MetafieldImportSummary::default(),
            items: vec![],
        };

        // Batch behavior parity: 10 items per batch, 1000ms between batches.
        let batch_size = 10usize;
        let total = parsed.len();
        let total_batches = total.div_ceil(batch_size);
        let mut processed = 0usize;

        for (batch_index, batch) in parsed.chunks(batch_size).enumerate() {
            let batch_number = batch_index + 1;
            logger.log(
                LogLevel::Info,
                "Processing batch",
                &[
                    LogField::new("batch", batch_number.to_string()),
                    LogField::new("batches_total", total_batches.to_string()),
                    LogField::new("items_in_batch", batch.len().to_string()),
                    LogField::new("processed_so_far", processed.to_string()),
                    LogField::new("total", total.to_string()),
                ],
            );

            for cfg in batch {
                report.summary.total += 1;
                processed += 1;

                let key = metafield_key(&cfg.namespace, &cfg.key);
                let existing = existing_map.get(&key);

                if let Some(ex) = existing {
                    if ex.type_name != cfg.type_name {
                        // recreate
                        if !options.allow_type_changes {
                            report.summary.failed += 1;
                            report.items.push(MetafieldImportItemReport {
                                namespace: cfg.namespace.clone(),
                                key: cfg.key.clone(),
                                action: MetafieldImportAction::Failed,
                                message: Some(
                                    "Type differs; recreation blocked (pass --allow-type-changes)"
                                        .to_string(),
                                ),
                            });
                            logger.log(
                                LogLevel::Info,
                                "Failed",
                                &[
                                    LogField::new(
                                        "definition",
                                        format!("{}.{}", cfg.namespace, cfg.key),
                                    ),
                                    LogField::new("reason", "type_change_blocked"),
                                ],
                            );
                            continue;
                        }

                        if (cfg.type_name == "reference" || cfg.type_name == "metaobject")
                            && !options.allow_associated_metafields_deletion
                        {
                            report.summary.failed += 1;
                            report.items.push(MetafieldImportItemReport {
                                namespace: cfg.namespace.clone(),
                                key: cfg.key.clone(),
                                action: MetafieldImportAction::Failed,
                                message: Some(
                                    "Type requires associated metafields deletion (pass --allow-associated-metafields-deletion)"
                                        .to_string(),
                                ),
                            });
                            logger.log(
                                LogLevel::Info,
                                "Failed",
                                &[
                                    LogField::new(
                                        "definition",
                                        format!("{}.{}", cfg.namespace, cfg.key),
                                    ),
                                    LogField::new(
                                        "reason",
                                        "requires_associated_metafields_deletion_flag",
                                    ),
                                ],
                            );
                            continue;
                        }

                        let id = ex.id.clone().ok_or_else(|| {
                            AppError::Gateway(format!(
                                "missing metafield definition id for existing {key}"
                            ))
                        })?;

                        self.gateway.metafield_definition_delete(&id, logger)?;
                        self.clock.sleep_millis(1000);

                        let validations =
                            validations_to_input(cfg.validations.clone(), &metaobject_type_to_id);
                        let input = MetafieldDefinitionInput {
                            namespace: cfg.namespace.clone(),
                            key: cfg.key.clone(),
                            name: cfg.name.clone(),
                            description: normalize_description(cfg.description.clone()),
                            type_name: cfg.type_name.clone(),
                            owner_type: owner_type.as_str().to_string(),
                            validations,
                            pin: cfg.pin,
                            capabilities: capabilities_to_input(cfg.capabilities.clone()),
                        };
                        self.gateway.metafield_definition_create(&input, logger)?;

                        report.summary.recreated += 1;
                        report.items.push(MetafieldImportItemReport {
                            namespace: cfg.namespace.clone(),
                            key: cfg.key.clone(),
                            action: MetafieldImportAction::Recreate,
                            message: None,
                        });
                        logger.log(
                            LogLevel::Info,
                            "Recreated",
                            &[LogField::new(
                                "definition",
                                format!("{}.{}", cfg.namespace, cfg.key),
                            )],
                        );
                        continue;
                    }

                    let validations =
                        validations_to_input(cfg.validations.clone(), &metaobject_type_to_id);

                    if should_update(cfg, &validations, ex) {
                        let id = ex.id.clone().ok_or_else(|| {
                            AppError::Gateway(format!(
                                "missing metafield definition id for existing {key}"
                            ))
                        })?;

                        let input = MetafieldDefinitionInput {
                            namespace: cfg.namespace.clone(),
                            key: cfg.key.clone(),
                            name: cfg.name.clone(),
                            description: normalize_description(cfg.description.clone()),
                            type_name: cfg.type_name.clone(),
                            owner_type: owner_type.as_str().to_string(),
                            validations,
                            pin: cfg.pin,
                            capabilities: capabilities_to_input(cfg.capabilities.clone()),
                        };
                        self.gateway
                            .metafield_definition_update(&id, &input, logger)?;

                        report.summary.updated += 1;
                        report.items.push(MetafieldImportItemReport {
                            namespace: cfg.namespace.clone(),
                            key: cfg.key.clone(),
                            action: MetafieldImportAction::Update,
                            message: None,
                        });
                        logger.log(
                            LogLevel::Info,
                            "Updated",
                            &[LogField::new(
                                "definition",
                                format!("{}.{}", cfg.namespace, cfg.key),
                            )],
                        );
                    } else {
                        report.summary.no_change += 1;
                        report.items.push(MetafieldImportItemReport {
                            namespace: cfg.namespace.clone(),
                            key: cfg.key.clone(),
                            action: MetafieldImportAction::NoChange,
                            message: None,
                        });
                        logger.log(
                            LogLevel::Info,
                            "No change",
                            &[LogField::new(
                                "definition",
                                format!("{}.{}", cfg.namespace, cfg.key),
                            )],
                        );
                    }
                } else {
                    // create
                    let validations =
                        validations_to_input(cfg.validations.clone(), &metaobject_type_to_id);
                    let input = MetafieldDefinitionInput {
                        namespace: cfg.namespace.clone(),
                        key: cfg.key.clone(),
                        name: cfg.name.clone(),
                        description: normalize_description(cfg.description.clone()),
                        type_name: cfg.type_name.clone(),
                        owner_type: owner_type.as_str().to_string(),
                        validations,
                        pin: cfg.pin,
                        capabilities: capabilities_to_input(cfg.capabilities.clone()),
                    };
                    match self.gateway.metafield_definition_create(&input, logger) {
                        Ok(()) => {
                            report.summary.created += 1;
                            report.items.push(MetafieldImportItemReport {
                                namespace: cfg.namespace.clone(),
                                key: cfg.key.clone(),
                                action: MetafieldImportAction::Create,
                                message: None,
                            });
                            logger.log(
                                LogLevel::Info,
                                "Created",
                                &[LogField::new(
                                    "definition",
                                    format!("{}.{}", cfg.namespace, cfg.key),
                                )],
                            );
                        }
                        Err(e) => {
                            report.summary.failed += 1;
                            report.items.push(MetafieldImportItemReport {
                                namespace: cfg.namespace.clone(),
                                key: cfg.key.clone(),
                                action: MetafieldImportAction::Failed,
                                message: Some(e.to_string()),
                            });
                            logger.log(
                                LogLevel::Info,
                                "Failed",
                                &[
                                    LogField::new(
                                        "definition",
                                        format!("{}.{}", cfg.namespace, cfg.key),
                                    ),
                                    LogField::new("error", e.to_string()),
                                ],
                            );
                        }
                    }
                }
            }

            // Delay between batches to respect rate limits (parity with Node).
            if batch_number < total_batches {
                logger.log(
                    LogLevel::Info,
                    "Waiting before next batch",
                    &[
                        LogField::new("millis", "1000"),
                        LogField::new("next_batch", (batch_number + 1).to_string()),
                    ],
                );
                self.clock.sleep_millis(1000);
            }
        }

        // Persist report.
        let ts = self.clock.now_timestamp_millis();
        let report_path = format!(
            "reports/metafield-definitions:import/metafield-import-report-{}.json",
            ts
        );
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| AppError::Json(e.to_string()))?;
        repo.write_text(&report_path, &out)?;

        Ok(report)
    }
}
