//! Metaobject export DTOs (read-model + exported JSON model).
//!
//! Pattern matches `metafields/export.rs`:
//! - A "Shopify read-model" represents what the gateway returns (rich, includes ids, etc.)
//! - An "exported model" represents what we write to `definitions/metaobjects.json`

use std::collections::HashMap;

use super::types::{
    MetaobjectAccessConfig, MetaobjectCapabilitiesConfig, MetaobjectFieldDefinitionConfig,
    MetaobjectValidationRule,
};
use crate::error::AppError;
use crate::logging::Logger;
use crate::ports::{FileRepo, MetaobjectGateway};
use serde::{Deserialize, Serialize};

/// Read-model for metaobject definition returned by a gateway (Shopify).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopifyMetaobjectDefinition {
    /// Shopify GID (not written to config, but useful for debugging / future import).
    #[serde(default)]
    pub id: Option<String>,

    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "displayNameKey", default)]
    pub display_name_key: Option<String>,

    #[serde(default)]
    pub access: Option<MetaobjectAccessConfig>,
    #[serde(default)]
    pub capabilities: Option<MetaobjectCapabilitiesConfig>,

    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<ShopifyMetaobjectFieldDefinition>,
}

/// Read-model for metaobject field definition returned by a gateway (Shopify).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopifyMetaobjectFieldDefinition {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub validations: Vec<MetaobjectValidationRule>,
}

/// Shape we export to `definitions/metaobjects.json`.
///
/// Mirrors Node exporter behavior: optional fields are omitted when `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedMetaobjectDefinition {
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,

    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<ExportedMetaobjectFieldDefinition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "displayNameKey", skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<MetaobjectAccessConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<MetaobjectCapabilitiesConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedMetaobjectFieldDefinition {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<MetaobjectValidationRule>>,
}

impl From<MetaobjectFieldDefinitionConfig> for ExportedMetaobjectFieldDefinition {
    fn from(v: MetaobjectFieldDefinitionConfig) -> Self {
        Self {
            key: v.key,
            name: v.name,
            type_name: v.type_name,
            required: v.required,
            description: v.description,
            validations: v.validations,
        }
    }
}

/// Transform Shopify metaobject definitions into exported JSON definitions.
///
/// Why this exists:
/// - keeps **business-level export rules** pure and testable (no HTTP, no filesystem)
/// - makes the use-case a simple orchestration: fetch -> transform -> write file
/// - ensures stable output shape for version control / cross-environment portability
///
/// Key rules (Node parity / as-is spec):
/// - filter out Shopify system metaobjects: `type.starts_with("shopify--")`
/// - convert `metaobject_definition_id` validations to `metaobject_definition_type` when mapping exists
/// - omit empty validations (export should prefer `null`/missing over empty arrays)
pub fn export_metaobject_definitions(
    input: Vec<ShopifyMetaobjectDefinition>,
    metaobject_id_to_type: &HashMap<String, String>,
) -> Vec<ExportedMetaobjectDefinition> {
    input
        .into_iter()
        .filter(|d| !d.type_name.starts_with("shopify--"))
        .map(|d| {
            let field_definitions = d
                .field_definitions
                .into_iter()
                .map(|f| {
                    let validations = if f.validations.is_empty() {
                        None
                    } else {
                        Some(
                            f.validations
                                .into_iter()
                                .map(|v| {
                                    if v.name == "metaobject_definition_id" {
                                        if let Some(id) = v.value.as_deref() {
                                            if let Some(t) = metaobject_id_to_type.get(id) {
                                                return MetaobjectValidationRule {
                                                    name: "metaobject_definition_type".to_string(),
                                                    value: Some(t.clone()),
                                                };
                                            }
                                        }
                                    }
                                    v
                                })
                                .collect(),
                        )
                    };

                    ExportedMetaobjectFieldDefinition {
                        key: f.key,
                        name: f.name,
                        type_name: f.type_name,
                        required: f.required,
                        description: f.description,
                        validations,
                    }
                })
                .collect::<Vec<_>>();

            ExportedMetaobjectDefinition {
                type_name: d.type_name,
                name: d.name,
                field_definitions,
                description: d.description,
                display_name_key: d.display_name_key,
                access: d.access,
                capabilities: d.capabilities,
            }
        })
        .collect()
}

pub struct ExportMetaobjectsUseCase<G> {
    gateway: G,
}

impl<G> ExportMetaobjectsUseCase<G>
where
    G: MetaobjectGateway,
{
    pub fn new(gateway: G) -> Self {
        Self { gateway }
    }

    pub fn execute(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<ExportedMetaobjectDefinition>, AppError> {
        let defs = self.gateway.list_metaobject_definitions(logger)?;

        // Build `id -> type` mapping from fetched definitions (needed for validation conversion).
        let id_to_type: HashMap<String, String> = defs
            .iter()
            .filter_map(|d| d.id.as_ref().map(|id| (id.clone(), d.type_name.clone())))
            .collect();

        Ok(export_metaobject_definitions(defs, &id_to_type))
    }
}

pub struct ExportMetaobjectsToFileUseCase<G> {
    gateway: G,
}

impl<G> ExportMetaobjectsToFileUseCase<G>
where
    G: MetaobjectGateway,
{
    pub fn new(gateway: G) -> Self {
        Self { gateway }
    }

    /// Export metaobject definitions and write JSON to:
    /// `definitions/metaobjects.json` (relative to cwd).
    pub fn execute(
        &self,
        repo: &mut impl FileRepo,
        logger: &dyn Logger,
    ) -> Result<Vec<ExportedMetaobjectDefinition>, AppError> {
        let defs = self.gateway.list_metaobject_definitions(logger)?;
        let id_to_type: HashMap<String, String> = defs
            .iter()
            .filter_map(|d| d.id.as_ref().map(|id| (id.clone(), d.type_name.clone())))
            .collect();

        let exported = export_metaobject_definitions(defs, &id_to_type);
        let json =
            serde_json::to_string_pretty(&exported).map_err(|e| AppError::Json(e.to_string()))?;
        repo.write_text("definitions/metaobjects.json", &json)?;
        Ok(exported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::{LogField, LogLevel};

    struct FakeMetaobjectGateway {
        items: Vec<ShopifyMetaobjectDefinition>,
    }

    impl MetaobjectGateway for FakeMetaobjectGateway {
        fn list_metaobject_definitions(
            &self,
            _logger: &dyn Logger,
        ) -> Result<Vec<ShopifyMetaobjectDefinition>, AppError> {
            Ok(self.items.clone())
        }
    }

    struct NoopLogger;
    impl Logger for NoopLogger {
        fn log(&self, _level: LogLevel, _message: &str, _fields: &[LogField]) {}
    }

    #[test]
    fn export_filters_out_shopify_types() {
        let input = vec![
            ShopifyMetaobjectDefinition {
                id: Some("gid://shopify/MetaobjectDefinition/1".to_string()),
                type_name: "shopify--system".to_string(),
                name: "System".to_string(),
                description: None,
                display_name_key: None,
                access: None,
                capabilities: None,
                field_definitions: vec![],
            },
            ShopifyMetaobjectDefinition {
                id: Some("gid://shopify/MetaobjectDefinition/2".to_string()),
                type_name: "custom_type".to_string(),
                name: "Custom".to_string(),
                description: None,
                display_name_key: None,
                access: None,
                capabilities: None,
                field_definitions: vec![ShopifyMetaobjectFieldDefinition {
                    key: "title".to_string(),
                    name: "Title".to_string(),
                    type_name: "single_line_text_field".to_string(),
                    required: true,
                    description: None,
                    validations: vec![],
                }],
            },
        ];

        let id_to_type = HashMap::from([
            (
                "gid://shopify/MetaobjectDefinition/1".to_string(),
                "shopify--system".to_string(),
            ),
            (
                "gid://shopify/MetaobjectDefinition/2".to_string(),
                "custom_type".to_string(),
            ),
        ]);

        let out = export_metaobject_definitions(input, &id_to_type);

        assert_eq!(
            out,
            vec![ExportedMetaobjectDefinition {
                type_name: "custom_type".to_string(),
                name: "Custom".to_string(),
                field_definitions: vec![ExportedMetaobjectFieldDefinition {
                    key: "title".to_string(),
                    name: "Title".to_string(),
                    type_name: "single_line_text_field".to_string(),
                    required: true,
                    description: None,
                    validations: None,
                }],
                description: None,
                display_name_key: None,
                access: None,
                capabilities: None,
            }]
        );
    }

    #[test]
    fn export_converts_metaobject_definition_id_validation_to_type_when_mapping_exists() {
        let input = vec![ShopifyMetaobjectDefinition {
            id: Some("gid://shopify/MetaobjectDefinition/2".to_string()),
            type_name: "custom_type".to_string(),
            name: "Custom".to_string(),
            description: None,
            display_name_key: None,
            access: None,
            capabilities: None,
            field_definitions: vec![ShopifyMetaobjectFieldDefinition {
                key: "ref".to_string(),
                name: "Ref".to_string(),
                type_name: "metaobject_reference".to_string(),
                required: false,
                description: None,
                validations: vec![
                    MetaobjectValidationRule {
                        name: "metaobject_definition_id".to_string(),
                        value: Some("gid://shopify/MetaobjectDefinition/99".to_string()),
                    },
                    MetaobjectValidationRule {
                        name: "other".to_string(),
                        value: Some("x".to_string()),
                    },
                ],
            }],
        }];

        let id_to_type = HashMap::from([(
            "gid://shopify/MetaobjectDefinition/99".to_string(),
            "target_type".to_string(),
        )]);

        let out = export_metaobject_definitions(input, &id_to_type);

        assert_eq!(
            out[0].field_definitions[0].validations.as_ref().unwrap(),
            &vec![
                MetaobjectValidationRule {
                    name: "metaobject_definition_type".to_string(),
                    value: Some("target_type".to_string()),
                },
                MetaobjectValidationRule {
                    name: "other".to_string(),
                    value: Some("x".to_string()),
                },
            ]
        );
    }

    struct FakeFileRepo {
        writes: HashMap<String, String>,
    }

    impl FakeFileRepo {
        fn new() -> Self {
            Self {
                writes: HashMap::new(),
            }
        }
    }

    impl FileRepo for FakeFileRepo {
        fn read_text(&self, _path: &str) -> Result<String, AppError> {
            Err(AppError::Repo("not implemented".to_string()))
        }

        fn write_text(&mut self, path: &str, contents: &str) -> Result<(), AppError> {
            self.writes.insert(path.to_string(), contents.to_string());
            Ok(())
        }
    }

    #[test]
    fn usecase_export_metaobjects_writes_definitions_file() {
        let gateway = FakeMetaobjectGateway {
            items: vec![ShopifyMetaobjectDefinition {
                id: Some("gid://shopify/MetaobjectDefinition/2".to_string()),
                type_name: "custom_type".to_string(),
                name: "Custom".to_string(),
                description: None,
                display_name_key: None,
                access: None,
                capabilities: None,
                field_definitions: vec![],
            }],
        };
        let usecase = ExportMetaobjectsToFileUseCase::new(gateway);
        let mut repo = FakeFileRepo::new();
        let logger = NoopLogger;

        let out = usecase.execute(&mut repo, &logger).unwrap();
        assert_eq!(out.len(), 1);
        assert!(repo.writes.contains_key("definitions/metaobjects.json"));
    }
}
