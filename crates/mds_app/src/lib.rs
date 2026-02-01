//! Application layer (use-cases + ports).

pub mod config;
pub mod logging;
pub mod metafield_config;
pub mod validation;

pub mod error;
pub mod metafields;
pub mod metaobjects;
pub mod ports;

#[cfg(test)]
mod metafield_config_tests;

// Re-exports (keep external API stable).
pub use error::AppError;

pub use ports::{
    Clock, FileRepo, MetafieldGateway, MetafieldImportGateway, MetaobjectGateway,
    MetaobjectImportGateway, SystemClock,
};

pub use metafields::types::{
    CapabilityFlag, MetafieldAccess, MetafieldCapabilities, MetafieldDefinitionCapabilitiesInput,
    MetafieldDefinitionInput, MetafieldDefinitionValidationInput, MetafieldValidation,
    ShopifyMetafieldDefinition,
};

pub use metafields::export::{
    export_metafield_definitions, ExportMetafieldsToFileUseCase, ExportMetafieldsUseCase,
    ExportedMetafieldDefinition,
};

pub use metafields::import::{
    ImportMetafieldsFromFileUseCase, ImportMetafieldsOptions, MetafieldImportAction,
    MetafieldImportItemReport, MetafieldImportReport, MetafieldImportSummary,
};

pub use metaobjects::types::{
    MetaobjectAccessConfig, MetaobjectCapabilitiesConfig, MetaobjectDefinitionConfig,
    MetaobjectFieldDefinitionConfig, MetaobjectValidationRule,
};

pub use metaobjects::export::{
    export_metaobject_definitions, ExportMetaobjectsToFileUseCase, ExportMetaobjectsUseCase,
    ExportedMetaobjectDefinition, ExportedMetaobjectFieldDefinition, ShopifyMetaobjectDefinition,
    ShopifyMetaobjectFieldDefinition,
};

pub use metaobjects::import::{
    ImportMetaobjectsFromFileUseCase, MetaobjectImportPlan, MetaobjectImportReport,
    PlanMetaobjectsImportUseCase,
};
