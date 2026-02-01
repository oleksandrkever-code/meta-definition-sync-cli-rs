//! Shopify gateway adapter (implements app-layer ports).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mds_app::{
    config::StoreConfig, logging::Logger, AppError, MetafieldDefinitionInput, MetafieldGateway,
    MetafieldImportGateway, MetaobjectGateway, MetaobjectImportGateway, ShopifyMetafieldDefinition,
};
use mds_domain::OwnerType;

use crate::shopify::{client::ShopifyClient, metafields, metaobjects};

#[derive(Debug, Clone)]
pub struct ShopifyMetafieldGateway {
    client: ShopifyClient,
    metaobject_type_to_id_cache: Arc<Mutex<Option<HashMap<String, String>>>>,
    metaobject_defs_cache:
        Arc<Mutex<Option<Vec<mds_app::metaobjects::export::ShopifyMetaobjectDefinition>>>>,
}

impl ShopifyMetafieldGateway {
    pub fn new(config: &StoreConfig) -> Self {
        Self {
            client: ShopifyClient::new(config.shop_domain.clone(), config.access_token.clone()),
            metaobject_type_to_id_cache: Arc::new(Mutex::new(None)),
            metaobject_defs_cache: Arc::new(Mutex::new(None)),
        }
    }

    fn get_cached_metaobject_type_to_id_map(
        &self,
        logger: &dyn Logger,
    ) -> Result<HashMap<String, String>, AppError> {
        let mut guard = self
            .metaobject_type_to_id_cache
            .lock()
            .map_err(|_| AppError::Gateway("metaobject cache mutex poisoned".into()))?;
        if let Some(v) = guard.as_ref() {
            return Ok(v.clone());
        }
        let fetched = metaobjects::fetch_metaobject_type_to_id_map(&self.client, logger)?;
        *guard = Some(fetched.clone());
        Ok(fetched)
    }

    fn get_cached_metaobject_definitions(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<mds_app::metaobjects::export::ShopifyMetaobjectDefinition>, AppError> {
        let mut guard = self
            .metaobject_defs_cache
            .lock()
            .map_err(|_| AppError::Gateway("metaobject cache mutex poisoned".into()))?;
        if let Some(v) = guard.as_ref() {
            return Ok(v.clone());
        }
        let fetched = metaobjects::list_metaobject_definitions_full(&self.client, logger)?;
        *guard = Some(fetched.clone());
        Ok(fetched)
    }

    fn reset_metaobject_cache_inner(&self) -> Result<(), AppError> {
        {
            let mut guard = self
                .metaobject_type_to_id_cache
                .lock()
                .map_err(|_| AppError::Gateway("metaobject cache mutex poisoned".into()))?;
            *guard = None;
        }
        {
            let mut guard = self
                .metaobject_defs_cache
                .lock()
                .map_err(|_| AppError::Gateway("metaobject cache mutex poisoned".into()))?;
            *guard = None;
        }
        Ok(())
    }
}

impl MetafieldGateway for ShopifyMetafieldGateway {
    fn list_metafield_definitions(
        &self,
        owner_type: OwnerType,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetafieldDefinition>, AppError> {
        metafields::list_metafield_definitions(&self.client, owner_type, logger)
    }
}

impl MetafieldImportGateway for ShopifyMetafieldGateway {
    fn list_existing_metafield_definitions(
        &self,
        owner_type: OwnerType,
        logger: &dyn Logger,
    ) -> Result<Vec<ShopifyMetafieldDefinition>, AppError> {
        // Reuse list (includes id).
        metafields::list_metafield_definitions(&self.client, owner_type, logger)
    }

    fn metaobject_type_to_id_map(
        &self,
        logger: &dyn Logger,
    ) -> Result<HashMap<String, String>, AppError> {
        self.get_cached_metaobject_type_to_id_map(logger)
    }

    fn metafield_definition_create(
        &self,
        input: &MetafieldDefinitionInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError> {
        metafields::metafield_definition_create(&self.client, input, logger)
    }

    fn metafield_definition_update(
        &self,
        id: &str,
        input: &MetafieldDefinitionInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError> {
        metafields::metafield_definition_update(&self.client, id, input, logger)
    }

    fn metafield_definition_delete(&self, id: &str, logger: &dyn Logger) -> Result<(), AppError> {
        metafields::metafield_definition_delete(&self.client, id, logger)
    }
}

impl MetaobjectGateway for ShopifyMetafieldGateway {
    fn list_metaobject_definitions(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<mds_app::metaobjects::export::ShopifyMetaobjectDefinition>, AppError> {
        self.get_cached_metaobject_definitions(logger)
    }
}

impl MetaobjectImportGateway for ShopifyMetafieldGateway {
    fn list_existing_metaobject_definitions(
        &self,
        logger: &dyn Logger,
    ) -> Result<Vec<mds_app::metaobjects::export::ShopifyMetaobjectDefinition>, AppError> {
        self.get_cached_metaobject_definitions(logger)
    }

    fn metaobject_type_to_id_map(
        &self,
        logger: &dyn Logger,
    ) -> Result<HashMap<String, String>, AppError> {
        self.get_cached_metaobject_type_to_id_map(logger)
    }

    fn metaobject_definition_create(
        &self,
        input: &mds_app::metaobjects::import::MetaobjectDefinitionCreateInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError> {
        metaobjects::metaobject_definition_create(&self.client, input, logger)
    }

    fn metaobject_definition_update(
        &self,
        id: &str,
        input: &mds_app::metaobjects::import::MetaobjectDefinitionUpdateInput,
        logger: &dyn Logger,
    ) -> Result<(), AppError> {
        metaobjects::metaobject_definition_update(&self.client, id, input, logger)
    }

    fn reset_metaobject_cache(&self) {
        let _ = self.reset_metaobject_cache_inner();
    }
}
