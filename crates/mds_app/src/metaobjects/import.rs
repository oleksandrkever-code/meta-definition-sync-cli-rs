//! Metaobject import (planning + execution).
//!
//! Parity notes (Node importer):
//! - Read `definitions/metaobjects.json`
//! - Print dependency plan before mutating Shopify
//! - Import level-by-level; each level in batches (10), 1000ms between batches/levels
//! - Resolve validations: `metaobject_definition_type` -> `metaobject_definition_id` using a cached map
//! - Cache is reset between levels to pick up newly created definitions

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::logging::{LogField, LogLevel, Logger};
use crate::ports::{Clock, FileRepo, MetaobjectImportGateway};

use super::deps::{
    build_internal_deps_map, collect_external_dependency_types, compute_levels,
    normalize_description, normalize_display_name_key, normalize_validations,
    render_dependency_forest_text,
};
use super::export::{ShopifyMetaobjectDefinition, ShopifyMetaobjectFieldDefinition};
use super::types::{
    MetaobjectCapabilitiesConfig, MetaobjectDefinitionConfig, MetaobjectValidationRule,
};

fn parse_json_string_array(input: &str) -> Option<Vec<String>> {
    let v: Vec<String> = serde_json::from_str(input).ok()?;
    Some(v)
}

fn to_json_string_array(items: &[String]) -> Option<String> {
    serde_json::to_string(items).ok()
}

// -------------------------------
// Planning DTO
// -------------------------------

#[derive(Debug, Clone)]
pub struct MetaobjectImportPlan {
    pub tree_markdown: String,
    pub levels: Vec<Vec<MetaobjectDefinitionConfig>>,
    pub total: usize,
    pub missing_external_types: Vec<String>,
}

pub struct PlanMetaobjectsImportUseCase<G> {
    gateway: G,
}

impl<G> PlanMetaobjectsImportUseCase<G>
where
    G: MetaobjectImportGateway,
{
    pub fn new(gateway: G) -> Self {
        Self { gateway }
    }

    pub fn execute(
        &self,
        repo: &mut impl FileRepo,
        logger: &dyn Logger,
    ) -> Result<MetaobjectImportPlan, AppError> {
        let input_path = "definitions/metaobjects.json";
        let json = match repo.read_text(input_path) {
            Ok(v) => v,
            Err(AppError::Repo(msg)) if msg.contains("file not found:") => {
                logger.log(
                    LogLevel::Info,
                    "Hint: run command to generate missing definitions file",
                    &[
                        LogField::new("suggested_command", "mdsr-cli metaobject export"),
                        LogField::new("output", "definitions/metaobjects.json"),
                    ],
                );
                return Err(AppError::Repo(msg));
            }
            Err(e) => return Err(e),
        };

        let parsed =
            crate::validation::parse_json_with_path::<Vec<MetaobjectDefinitionConfig>>(&json)
                .map_err(|errs| {
                    AppError::Json(crate::validation::format_validation_errors(&errs))
                })?;

        if parsed.is_empty() {
            return Err(AppError::Json(format!(
                "metaobject config file must be a non-empty array: {input_path}"
            )));
        }

        let defs = parsed;

        let (internal_deps, internal_types) = build_internal_deps_map(&defs);
        let level_types = compute_levels(&internal_deps)?;

        // External deps: referenced in validations, but not present in JSON.
        let external_refs = collect_external_dependency_types(&defs, &internal_types);

        let by_type: HashMap<String, MetaobjectDefinitionConfig> =
            defs.into_iter().map(|d| (d.type_name.clone(), d)).collect();

        let mut levels: Vec<Vec<MetaobjectDefinitionConfig>> = vec![];
        for lt in &level_types {
            let mut lvl_defs: Vec<MetaobjectDefinitionConfig> = vec![];
            for t in lt {
                if let Some(d) = by_type.get(t) {
                    lvl_defs.push(d.clone());
                }
            }
            levels.push(lvl_defs);
        }

        // Determine which external refs exist in Shopify (via cached map).
        let type_to_id = self.gateway.metaobject_type_to_id_map(logger)?;
        let mut external_in_shopify: Vec<String> = vec![];
        let mut missing_external: Vec<String> = vec![];
        for t in external_refs {
            if type_to_id.contains_key(&t) {
                external_in_shopify.push(t);
            } else {
                missing_external.push(t);
            }
        }
        external_in_shopify.sort();
        missing_external.sort();

        // For rendering, we need deps map for all types (internal deps only).
        let tree_markdown =
            render_dependency_forest_text(&internal_deps, &external_in_shopify, &missing_external);

        if !missing_external.is_empty() {
            logger.log(
                LogLevel::Warn,
                "Some external metaobject dependencies are not present in Shopify",
                &[LogField::new(
                    "missing_external_types",
                    missing_external.join(", "),
                )],
            );
        }

        Ok(MetaobjectImportPlan {
            tree_markdown,
            levels,
            total: by_type.len(),
            missing_external_types: missing_external,
        })
    }
}

// -------------------------------
// Mutation input DTOs (sent to Shopify)
// -------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetaobjectFieldDefinitionCreateInput {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<MetaobjectValidationRule>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetaobjectFieldDefinitionUpdateInput {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<MetaobjectValidationRule>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaobjectDefinitionCreateInput {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "displayNameKey", skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<MetaobjectCapabilitiesConfig>,
    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<MetaobjectFieldDefinitionCreateInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaobjectDefinitionUpdateInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "displayNameKey", skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<MetaobjectCapabilitiesConfig>,
    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<MetaobjectFieldDefinitionOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MetaobjectFieldDefinitionOperation {
    #[serde(rename_all = "camelCase")]
    Create {
        create: MetaobjectFieldDefinitionCreateInput,
    },
    #[serde(rename_all = "camelCase")]
    Update {
        update: MetaobjectFieldDefinitionUpdateInput,
    },
}

fn resolve_validations_for_shopify(
    validations: &Option<Vec<MetaobjectValidationRule>>,
    type_to_id: &HashMap<String, String>,
    logger: &dyn Logger,
) -> Result<Option<Vec<MetaobjectValidationRule>>, AppError> {
    let v = normalize_validations(validations);
    if v.is_empty() {
        return Ok(None);
    }

    let id_set = type_to_id.values().cloned().collect::<std::collections::HashSet<_>>();

    let mut out: Vec<MetaobjectValidationRule> = vec![];
    for rule in v {
        if rule.name == "metaobject_definition_type" {
            if let Some(t) = rule.value.as_deref() {
                if let Some(id) = type_to_id.get(t) {
                    out.push(MetaobjectValidationRule {
                        name: "metaobject_definition_id".to_string(),
                        value: Some(id.clone()),
                    });
                    logger.log(
                        LogLevel::Debug,
                        "Resolved metaobject_definition_type -> metaobject_definition_id",
                        &[
                            LogField::new("type", t.to_string()),
                            LogField::new("id", id.clone()),
                        ],
                    );
                    continue;
                } else {
                    return Err(AppError::Json(format!(
                        "cannot resolve metaobject_definition_type `{}` in Shopify (type not found)",
                        t
                    )));
                }
            }
        }
        if rule.name == "metaobject_definition_types" {
            let raw = rule.value.as_deref().unwrap_or_default();
            let types = parse_json_string_array(raw).ok_or_else(|| {
                AppError::Json(format!(
                    "invalid metaobject_definition_types value (expected JSON array string): `{}`",
                    raw
                ))
            })?;

            let mut ids: Vec<String> = vec![];
            for t in &types {
                let t = t.trim();
                let id = type_to_id.get(t).ok_or_else(|| {
                    AppError::Json(format!(
                        "cannot resolve metaobject_definition_types item `{}` in Shopify (type not found)",
                        t
                    ))
                })?;
                ids.push(id.clone());
            }

            let json = to_json_string_array(&ids).ok_or_else(|| {
                AppError::Json("failed to serialize metaobject_definition_ids array".to_string())
            })?;

            out.push(MetaobjectValidationRule {
                name: "metaobject_definition_ids".to_string(),
                value: Some(json),
            });
            logger.log(
                LogLevel::Debug,
                "Resolved metaobject_definition_types -> metaobject_definition_ids",
                &[
                    LogField::new("types", types.join(", ")),
                    LogField::new("ids", ids.join(", ")),
                ],
            );
            continue;
        }
        // If user config contains store-specific IDs, detect and fail early with a clear message.
        if rule.name == "metaobject_definition_id" {
            if let Some(id) = rule.value.as_deref() {
                if !id_set.contains(id) {
                    return Err(AppError::Json(format!(
                        "metaobject_definition_id `{}` does not belong to the target Shopify store (export should use metaobject_definition_type instead)",
                        id
                    )));
                }
            }
        }
        if rule.name == "metaobject_definition_ids" {
            if let Some(raw) = rule.value.as_deref() {
                if let Some(ids) = parse_json_string_array(raw) {
                    for id in ids {
                        if !id_set.contains(&id) {
                            return Err(AppError::Json(format!(
                                "metaobject_definition_ids contains `{}` which does not belong to the target Shopify store (export should use metaobject_definition_types instead)",
                                id
                            )));
                        }
                    }
                } else {
                    return Err(AppError::Json(format!(
                        "invalid metaobject_definition_ids value (expected JSON array string): `{}`",
                        raw
                    )));
                }
            }
        }
        out.push(rule);
    }

    if out.is_empty() {
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

fn build_create_input(
    cfg: &MetaobjectDefinitionConfig,
    type_to_id: &HashMap<String, String>,
    logger: &dyn Logger,
) -> Result<MetaobjectDefinitionCreateInput, AppError> {
    let field_definitions = cfg
        .field_definitions
        .iter()
        .map(|f| {
            Ok(MetaobjectFieldDefinitionCreateInput {
                key: f.key.clone(),
                name: f.name.clone(),
                description: normalize_description(f.description.clone()),
                type_name: f.type_name.clone(),
                required: f.required,
                validations: resolve_validations_for_shopify(&f.validations, type_to_id, logger)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(MetaobjectDefinitionCreateInput {
        type_name: cfg.type_name.clone(),
        name: cfg.name.clone(),
        description: normalize_description(cfg.description.clone()),
        display_name_key: normalize_display_name_key(cfg.display_name_key.clone()),
        capabilities: cfg.capabilities.clone(),
        field_definitions,
    })
}

fn build_update_input(
    cfg: &MetaobjectDefinitionConfig,
    existing: &ShopifyMetaobjectDefinition,
    type_to_id: &HashMap<String, String>,
    logger: &dyn Logger,
) -> Result<MetaobjectDefinitionUpdateInput, AppError> {
    let mut existing_fields_by_key: HashMap<String, &ShopifyMetaobjectFieldDefinition> =
        HashMap::new();
    for f in &existing.field_definitions {
        existing_fields_by_key.insert(f.key.clone(), f);
    }

    let mut ops: Vec<MetaobjectFieldDefinitionOperation> = vec![];
    for f in &cfg.field_definitions {
        if existing_fields_by_key.contains_key(&f.key) {
            ops.push(MetaobjectFieldDefinitionOperation::Update {
                update: MetaobjectFieldDefinitionUpdateInput {
                    key: f.key.clone(),
                    name: f.name.clone(),
                    description: normalize_description(f.description.clone()),
                    required: f.required,
                    validations: resolve_validations_for_shopify(&f.validations, type_to_id, logger)?,
                },
            });
        } else {
            ops.push(MetaobjectFieldDefinitionOperation::Create {
                create: MetaobjectFieldDefinitionCreateInput {
                    key: f.key.clone(),
                    name: f.name.clone(),
                    description: normalize_description(f.description.clone()),
                    type_name: f.type_name.clone(),
                    required: f.required,
                    validations: resolve_validations_for_shopify(&f.validations, type_to_id, logger)?,
                },
            });
        }
    }

    Ok(MetaobjectDefinitionUpdateInput {
        name: cfg.name.clone(),
        description: normalize_description(cfg.description.clone()),
        display_name_key: normalize_display_name_key(cfg.display_name_key.clone()),
        capabilities: cfg.capabilities.clone(),
        field_definitions: ops,
    })
}

fn normalize_existing_field_desc(d: &Option<String>) -> Option<String> {
    normalize_description(d.clone())
}

fn validations_equal(
    a: &Option<Vec<MetaobjectValidationRule>>,
    b: &Vec<MetaobjectValidationRule>,
) -> bool {
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

fn has_changes(
    existing: &ShopifyMetaobjectDefinition,
    cfg: &MetaobjectDefinitionConfig,
    type_to_id: &HashMap<String, String>,
    logger: &dyn Logger,
) -> bool {
    if existing.name != cfg.name {
        return true;
    }

    if normalize_description(existing.description.clone())
        != normalize_description(cfg.description.clone())
    {
        return true;
    }

    if normalize_display_name_key(existing.display_name_key.clone())
        != normalize_display_name_key(cfg.display_name_key.clone())
    {
        return true;
    }

    // Access comparison intentionally skipped (Node parity).
    // Capabilities comparison skipped for now (Node parity).

    if existing.field_definitions.len() != cfg.field_definitions.len() {
        return true;
    }

    let mut existing_by_key: HashMap<String, &ShopifyMetaobjectFieldDefinition> = HashMap::new();
    for f in &existing.field_definitions {
        existing_by_key.insert(f.key.clone(), f);
    }

    let mut cfg_by_key: HashMap<String, &super::types::MetaobjectFieldDefinitionConfig> =
        HashMap::new();
    for f in &cfg.field_definitions {
        cfg_by_key.insert(f.key.clone(), f);
    }

    for (key, cfg_field) in cfg_by_key {
        let Some(ex_field) = existing_by_key.get(&key) else {
            return true;
        };

        let resolved_validations =
            resolve_validations_for_shopify(&cfg_field.validations, type_to_id, logger)
                .unwrap_or(None);

        if ex_field.name != cfg_field.name
            || normalize_existing_field_desc(&ex_field.description)
                != normalize_description(cfg_field.description.clone())
            || ex_field.type_name != cfg_field.type_name
            || ex_field.required != cfg_field.required
            || !validations_equal(&resolved_validations, &ex_field.validations)
        {
            return true;
        }
    }

    // Existing fields missing in JSON would be deleted (we treat as change, Node parity)
    for key in existing_by_key.keys() {
        if !cfg.field_definitions.iter().any(|f| &f.key == key) {
            return true;
        }
    }

    false
}

// -------------------------------
// Import reporting
// -------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaobjectImportAction {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "no_change")]
    NoChange,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaobjectImportItemReport {
    #[serde(rename = "type")]
    pub type_name: String,
    pub action: MetaobjectImportAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MetaobjectImportSummary {
    pub created: usize,
    pub updated: usize,
    #[serde(rename = "noChange")]
    pub no_change: usize,
    pub failed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaobjectImportReport {
    pub summary: MetaobjectImportSummary,
    pub items: Vec<MetaobjectImportItemReport>,
}

// -------------------------------
// Execution use-case
// -------------------------------

pub struct ImportMetaobjectsFromFileUseCase<G, C> {
    gateway: G,
    clock: C,
}

impl<G, C> ImportMetaobjectsFromFileUseCase<G, C>
where
    G: MetaobjectImportGateway,
    C: Clock,
{
    pub fn new(gateway: G, clock: C) -> Self {
        Self { gateway, clock }
    }

    pub fn execute(
        &self,
        plan: &MetaobjectImportPlan,
        repo: &mut impl FileRepo,
        logger: &dyn Logger,
    ) -> Result<MetaobjectImportReport, AppError> {
        let mut report = MetaobjectImportReport {
            summary: MetaobjectImportSummary {
                total: plan.total,
                ..Default::default()
            },
            items: vec![],
        };

        // Cache should be used within one level. Between levels we reset it (Node parity).
        let batch_size = 10usize;
        let delay_ms = 1000u64;

        // Existing definitions cache per level (refreshed between levels).
        let mut existing_by_type: HashMap<String, ShopifyMetaobjectDefinition> = HashMap::new();

        for (level_idx, level_defs) in plan.levels.iter().enumerate() {
            // Refresh existing defs at the start of each level.
            existing_by_type.clear();
            for d in self.gateway.list_existing_metaobject_definitions(logger)? {
                existing_by_type.insert(d.type_name.clone(), d);
            }

            let type_to_id = self.gateway.metaobject_type_to_id_map(logger)?;

            logger.log(
                LogLevel::Info,
                "Processing import level",
                &[
                    LogField::new("level", level_idx.to_string()),
                    LogField::new("items", level_defs.len().to_string()),
                ],
            );

            let total = level_defs.len();
            let total_batches = total.div_ceil(batch_size);

            for (batch_index, batch) in level_defs.chunks(batch_size).enumerate() {
                let batch_number = batch_index + 1;
                logger.log(
                    LogLevel::Info,
                    "Processing batch",
                    &[
                        LogField::new("level", level_idx.to_string()),
                        LogField::new("batch", format!("{batch_number}/{total_batches}")),
                        LogField::new("batch_size", batch.len().to_string()),
                    ],
                );

                for cfg in batch {
                    let t = cfg.type_name.clone();
                    let existing = existing_by_type.get(&t);
                    let mut item = MetaobjectImportItemReport {
                        type_name: t.clone(),
                        action: MetaobjectImportAction::Create,
                        message: None,
                    };

                    let res: Result<(), AppError> = match existing {
                        Some(ex) => {
                            if has_changes(ex, cfg, &type_to_id, logger) {
                                let id = ex.id.as_deref().ok_or_else(|| {
                                    AppError::Gateway(format!(
                                        "existing metaobject definition missing id for type `{}`",
                                        t
                                    ))
                                })?;
                                let input = build_update_input(cfg, ex, &type_to_id, logger)?;
                                self.gateway
                                    .metaobject_definition_update(id, &input, logger)?;
                                item.action = MetaobjectImportAction::Update;
                                Ok(())
                            } else {
                                item.action = MetaobjectImportAction::NoChange;
                                Ok(())
                            }
                        }
                        None => {
                            let input = build_create_input(cfg, &type_to_id, logger)?;
                            self.gateway.metaobject_definition_create(&input, logger)?;
                            item.action = MetaobjectImportAction::Create;
                            Ok(())
                        }
                    };

                    match res {
                        Ok(()) => {
                            match item.action {
                                MetaobjectImportAction::Create => report.summary.created += 1,
                                MetaobjectImportAction::Update => report.summary.updated += 1,
                                MetaobjectImportAction::NoChange => report.summary.no_change += 1,
                                MetaobjectImportAction::Failed => {}
                            }
                            logger.log(
                                LogLevel::Info,
                                "Metaobject processed",
                                &[
                                    LogField::new("type", item.type_name.clone()),
                                    LogField::new("action", format!("{:?}", item.action)),
                                ],
                            );
                        }
                        Err(e) => {
                            report.summary.failed += 1;
                            item.action = MetaobjectImportAction::Failed;
                            item.message = Some(e.to_string());
                            logger.log(
                                LogLevel::Error,
                                "Metaobject failed",
                                &[
                                    LogField::new("type", item.type_name.clone()),
                                    LogField::new("error", e.to_string()),
                                ],
                            );
                        }
                    }

                    report.items.push(item);
                }

                // Rate limiting between batches inside a level.
                if batch_number < total_batches {
                    logger.log(
                        LogLevel::Info,
                        "Waiting before next batch",
                        &[
                            LogField::new("level", level_idx.to_string()),
                            LogField::new("sleep_ms", delay_ms.to_string()),
                        ],
                    );
                    self.clock.sleep_millis(delay_ms);
                }
            }

            // Between levels: reset cache + wait + continue.
            if level_idx + 1 < plan.levels.len() {
                logger.log(
                    LogLevel::Info,
                    "Waiting before next level",
                    &[
                        LogField::new("next_level", (level_idx + 1).to_string()),
                        LogField::new("sleep_ms", delay_ms.to_string()),
                    ],
                );
                self.gateway.reset_metaobject_cache();
                self.clock.sleep_millis(delay_ms);
            }
        }

        // Persist report (parity with metafield import + Node importer).
        let ts = self.clock.now_timestamp_millis();
        let report_path = format!(
            "reports/metaobject-definitions:import/metaobject-import-report-{}.json",
            ts
        );
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| AppError::Json(e.to_string()))?;
        repo.write_text(&report_path, &out)?;

        Ok(report)
    }
}
