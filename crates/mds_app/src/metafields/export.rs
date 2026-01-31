//! Metafield export (application-layer transformation rules + use-cases).

use serde::{Deserialize, Serialize};

use mds_domain::OwnerType;

use crate::error::AppError;
use crate::logging::Logger;
use crate::ports::{FileRepo, MetafieldGateway};

use super::types::{
    MetafieldAccess, MetafieldCapabilities, MetafieldValidation, ShopifyMetafieldDefinition,
};

/// Minimal shape of metafield definition we export to JSON config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedMetafieldDefinition {
    pub namespace: String,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<MetafieldValidation>>,
    /// Always included (parity with Node): true if pinnedPosition != null.
    pub pin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<MetafieldAccess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<MetafieldCapabilities>,
}

/// Transform Shopify definitions into exported JSON definitions.
///
/// Rule (parity with Node): exclude definitions where namespace starts with `"shopify"`.
pub fn export_metafield_definitions(
    input: Vec<ShopifyMetafieldDefinition>,
) -> Vec<ExportedMetafieldDefinition> {
    input
        .into_iter()
        .filter(|d| !d.namespace.starts_with("shopify"))
        .map(|d| {
            // Normalize access values to match Node export behavior.
            let access = d.access.map(|mut a| {
                if let Some(admin) = a.admin.as_deref() {
                    let mapped = match admin {
                        "PUBLIC_READ_WRITE" => "MERCHANT_READ_WRITE",
                        "PUBLIC_READ" => "MERCHANT_READ",
                        "PRIVATE" => "MERCHANT_READ",
                        _ => admin,
                    };
                    a.admin = Some(mapped.to_string());
                }
                if let Some(storefront) = a.storefront.as_deref() {
                    let mapped = match storefront {
                        "MERCHANT_READ" => "PUBLIC_READ",
                        _ => storefront,
                    };
                    a.storefront = Some(mapped.to_string());
                }
                a
            });

            // Keep validations only if non-empty (parity with Node exporter which omits empty arrays).
            let validations = if d.validations.is_empty() {
                None
            } else {
                Some(d.validations)
            };

            ExportedMetafieldDefinition {
                namespace: d.namespace,
                key: d.key,
                name: d.name,
                description: d.description,
                type_name: d.type_name,
                validations,
                pin: d.pinned_position.is_some(),
                access,
                capabilities: d.capabilities,
            }
        })
        .collect()
}

pub struct ExportMetafieldsUseCase<G> {
    gateway: G,
}

impl<G> ExportMetafieldsUseCase<G>
where
    G: MetafieldGateway,
{
    pub fn new(gateway: G) -> Self {
        Self { gateway }
    }

    pub fn execute(
        &self,
        owner_type: OwnerType,
        logger: &dyn Logger,
    ) -> Result<Vec<ExportedMetafieldDefinition>, AppError> {
        let defs = self
            .gateway
            .list_metafield_definitions(owner_type, logger)?;
        Ok(export_metafield_definitions(defs))
    }
}

pub struct ExportMetafieldsToFileUseCase<G> {
    gateway: G,
}

impl<G> ExportMetafieldsToFileUseCase<G>
where
    G: MetafieldGateway,
{
    pub fn new(gateway: G) -> Self {
        Self { gateway }
    }

    /// Export one owner type and write JSON to:
    /// `definitions/metafields/<owner>.json` (relative to cwd).
    pub fn execute(
        &self,
        owner_type: OwnerType,
        repo: &mut impl FileRepo,
        logger: &dyn Logger,
    ) -> Result<Vec<ExportedMetafieldDefinition>, AppError> {
        let defs = self
            .gateway
            .list_metafield_definitions(owner_type, logger)?;
        let exported = export_metafield_definitions(defs);

        let owner = owner_type.as_str().to_ascii_lowercase();
        let path = format!("definitions/metafields/{owner}.json");
        let json = serde_json::to_string_pretty(&exported)
            .map_err(|e| AppError::Json(e.to_string()))?;
        repo.write_text(&path, &json)?;

        Ok(exported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::{LogField, LogLevel};
    use crate::metafields::types::ShopifyMetafieldDefinition;

    struct FakeMetafieldGateway {
        items: Vec<ShopifyMetafieldDefinition>,
    }

    impl MetafieldGateway for FakeMetafieldGateway {
        fn list_metafield_definitions(
            &self,
            _owner_type: OwnerType,
            _logger: &dyn Logger,
        ) -> Result<Vec<ShopifyMetafieldDefinition>, AppError> {
            Ok(self.items.clone())
        }
    }

    struct NoopLogger;
    impl Logger for NoopLogger {
        fn log(&self, _level: LogLevel, _message: &str, _fields: &[LogField]) {}
    }

    #[test]
    fn export_filters_out_shopify_namespaces() {
        let input = vec![
            ShopifyMetafieldDefinition {
                id: None,
                namespace: "shopify.test".to_string(),
                key: "a".to_string(),
                name: "A".to_string(),
                description: None,
                type_name: "single_line_text_field".to_string(),
                validations: vec![],
                pinned_position: None,
                access: None,
                capabilities: None,
            },
            ShopifyMetafieldDefinition {
                id: None,
                namespace: "custom".to_string(),
                key: "b".to_string(),
                name: "B".to_string(),
                description: None,
                type_name: "single_line_text_field".to_string(),
                validations: vec![],
                pinned_position: None,
                access: None,
                capabilities: None,
            },
        ];

        let out = export_metafield_definitions(input);

        assert_eq!(
            out,
            vec![ExportedMetafieldDefinition {
                namespace: "custom".to_string(),
                key: "b".to_string(),
                name: "B".to_string(),
                description: None,
                type_name: "single_line_text_field".to_string(),
                validations: None,
                pin: false,
                access: None,
                capabilities: None,
            }]
        );
    }

    #[test]
    fn usecase_export_metafields_filters_shopify_namespaces() {
        let gateway = FakeMetafieldGateway {
            items: vec![
                ShopifyMetafieldDefinition {
                    id: None,
                    namespace: "shopify.system".to_string(),
                    key: "x".to_string(),
                    name: "X".to_string(),
                    description: None,
                    type_name: "single_line_text_field".to_string(),
                    validations: vec![],
                    pinned_position: None,
                    access: None,
                    capabilities: None,
                },
                ShopifyMetafieldDefinition {
                    id: None,
                    namespace: "custom".to_string(),
                    key: "y".to_string(),
                    name: "Y".to_string(),
                    description: None,
                    type_name: "single_line_text_field".to_string(),
                    validations: vec![],
                    pinned_position: None,
                    access: None,
                    capabilities: None,
                },
            ],
        };

        let usecase = ExportMetafieldsUseCase::new(gateway);
        let out = usecase.execute(OwnerType::Product, &NoopLogger).unwrap();

        assert_eq!(
            out,
            vec![ExportedMetafieldDefinition {
                namespace: "custom".to_string(),
                key: "y".to_string(),
                name: "Y".to_string(),
                description: None,
                type_name: "single_line_text_field".to_string(),
                validations: None,
                pin: false,
                access: None,
                capabilities: None,
            }]
        );
    }

    #[derive(Default)]
    struct FakeFileRepo {
        writes: Vec<(String, String)>,
    }

    impl FileRepo for FakeFileRepo {
        fn read_text(&self, _path: &str) -> Result<String, AppError> {
            Err(AppError::Repo("not implemented in FakeFileRepo".into()))
        }

        fn write_text(&mut self, path: &str, contents: &str) -> Result<(), AppError> {
            self.writes.push((path.to_string(), contents.to_string()));
            Ok(())
        }
    }

    #[test]
    fn usecase_export_product_writes_definitions_file() {
        let gateway = FakeMetafieldGateway {
            items: vec![
                ShopifyMetafieldDefinition {
                    id: None,
                    namespace: "shopify.system".to_string(),
                    key: "skip".to_string(),
                    name: "Skip".to_string(),
                    description: None,
                    type_name: "single_line_text_field".to_string(),
                    validations: vec![],
                    pinned_position: None,
                    access: None,
                    capabilities: None,
                },
                ShopifyMetafieldDefinition {
                    id: None,
                    namespace: "custom".to_string(),
                    key: "keep".to_string(),
                    name: "Keep".to_string(),
                    description: None,
                    type_name: "single_line_text_field".to_string(),
                    validations: vec![],
                    pinned_position: None,
                    access: None,
                    capabilities: None,
                },
            ],
        };

        let usecase = ExportMetafieldsToFileUseCase::new(gateway);
        let mut repo = FakeFileRepo::default();

        let exported = usecase
            .execute(OwnerType::Product, &mut repo, &NoopLogger)
            .unwrap();

        assert_eq!(
            exported,
            vec![ExportedMetafieldDefinition {
                namespace: "custom".to_string(),
                key: "keep".to_string(),
                name: "Keep".to_string(),
                description: None,
                type_name: "single_line_text_field".to_string(),
                validations: None,
                pin: false,
                access: None,
                capabilities: None,
            }]
        );

        assert_eq!(repo.writes.len(), 1);
        assert_eq!(repo.writes[0].0, "definitions/metafields/product.json");

        let parsed: Vec<ExportedMetafieldDefinition> =
            serde_json::from_str(&repo.writes[0].1).unwrap();
        assert_eq!(parsed, exported);
    }
}

