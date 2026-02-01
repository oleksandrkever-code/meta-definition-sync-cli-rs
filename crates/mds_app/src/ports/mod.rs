//! Application-layer ports (interfaces).

use std::collections::HashMap;

use mds_domain::OwnerType;

use crate::error::AppError;
use crate::logging::Logger;
use crate::metafields::types::{MetafieldDefinitionInput, ShopifyMetafieldDefinition};
use crate::metaobjects::export::ShopifyMetaobjectDefinition;
use crate::metaobjects::import::{
    MetaobjectDefinitionCreateInput, MetaobjectDefinitionUpdateInput,
};

pub trait MetafieldGateway {
    fn list_metafield_definitions(
        &self,
        owner_type: OwnerType,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetafieldDefinition>, AppError>;
}

pub trait MetaobjectGateway {
    fn list_metaobject_definitions(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetaobjectDefinition>, AppError>;
}

pub trait MetaobjectImportGateway {
    fn list_existing_metaobject_definitions(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetaobjectDefinition>, AppError>;

    /// Map metaobject definition type -> Shopify GID.
    fn metaobject_type_to_id_map(
        &self,
        logger: &dyn Logger,
    ) -> Result<HashMap<String, String>, AppError>;

    fn metaobject_definition_create(
        &self,
        input: &MetaobjectDefinitionCreateInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError>;

    fn metaobject_definition_update(
        &self,
        id: &str,
        input: &MetaobjectDefinitionUpdateInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError>;

    /// Reset per-run caches related to metaobject definitions (type<->id, existing defs, etc).
    fn reset_metaobject_cache(&self);
}

pub trait MetafieldImportGateway {
    fn list_existing_metafield_definitions(
        &self,
        owner_type: OwnerType,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetafieldDefinition>, AppError>;

    /// Map metaobject definition type -> Shopify GID.
    fn metaobject_type_to_id_map(
        &self,
        logger: &dyn Logger,
    ) -> Result<HashMap<String, String>, AppError>;

    fn metafield_definition_create(
        &self,
        input: &MetafieldDefinitionInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError>;

    fn metafield_definition_update(
        &self,
        id: &str,
        input: &MetafieldDefinitionInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError>;

    fn metafield_definition_delete(&self, id: &str, logger: &dyn Logger) -> Result<(), AppError>;
}

pub trait FileRepo {
    fn read_text(&self, path: &str) -> Result<String, AppError>;
    fn write_text(&mut self, path: &str, contents: &str) -> Result<(), AppError>;
}

pub trait Clock {
    fn now_timestamp_millis(&self) -> u128;
    fn sleep_millis(&self, millis: u64);
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_timestamp_millis(&self) -> u128 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }

    fn sleep_millis(&self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }
}
