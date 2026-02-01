//! Metaobject-related Shopify operations (infra).

use std::collections::HashMap;

use mds_app::metaobjects::export::{ShopifyMetaobjectDefinition, ShopifyMetaobjectFieldDefinition};
use mds_app::{
    logging::{LogField, LogLevel, Logger},
    AppError, CapabilityFlag, MetaobjectAccessConfig, MetaobjectCapabilitiesConfig,
    MetaobjectValidationRule,
};

use crate::shopify::dto::metaobjects::MetaobjectDefsDataForExport;
use crate::shopify::dto::{format_user_errors, metaobjects::*};
use crate::shopify::{
    client::ShopifyClient,
    dto::{metaobjects::MetaobjectDefsData, metaobjects::MetaobjectDefsVars},
    graphql,
};

pub fn fetch_metaobject_id_to_type_map(
    client: &ShopifyClient,
    logger: &dyn Logger,
) -> Result<HashMap<String, String>, AppError> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut after: Option<String> = None;

    loop {
        let vars = MetaobjectDefsVars {
            first: 50,
            after: after.clone(),
        };

        logger.log(
            LogLevel::Debug,
            "GraphQL metaobjectDefinitions vars",
            &[
                LogField::new("first", vars.first.to_string()),
                LogField::new("after", format!("{:?}", vars.after)),
            ],
        );

        let body = client.post_graphql::<_, MetaobjectDefsData>(
            graphql::GET_METAOBJECT_DEFINITIONS_QUERY,
            vars,
            logger,
        )?;

        if let Some(errs) = body.errors {
            return Err(AppError::Gateway(format!(
                "GraphQL errors when querying metaobjectDefinitions: {errs}"
            )));
        }

        let data = body.data.ok_or_else(|| {
            AppError::Gateway("missing data in metaobjectDefinitions response".into())
        })?;

        for edge in data.metaobject_definitions.edges {
            out.insert(edge.node.id, edge.node.r#type);
        }

        if data.metaobject_definitions.page_info.has_next_page {
            after = data.metaobject_definitions.page_info.end_cursor;
            if after.is_none() {
                break;
            }
        } else {
            break;
        }
    }

    Ok(out)
}

pub fn list_metaobject_definitions_full(
    client: &ShopifyClient,
    logger: &dyn Logger,
) -> Result<Vec<ShopifyMetaobjectDefinition>, AppError> {
    let mut out: Vec<ShopifyMetaobjectDefinition> = vec![];
    let mut after: Option<String> = None;

    loop {
        let vars = MetaobjectDefsVars {
            first: 50,
            after: after.clone(),
        };

        logger.log(
            LogLevel::Debug,
            "GraphQL metaobjectDefinitions vars",
            &[
                LogField::new("first", vars.first.to_string()),
                LogField::new("after", format!("{:?}", vars.after)),
            ],
        );

        let body = client.post_graphql::<_, MetaobjectDefsDataForExport>(
            graphql::GET_METAOBJECT_DEFINITIONS_EXPORT_QUERY,
            vars,
            logger,
        )?;

        if let Some(errs) = body.errors {
            return Err(AppError::Gateway(format!(
                "GraphQL errors when querying metaobjectDefinitions: {errs}"
            )));
        }

        let data = body.data.ok_or_else(|| {
            AppError::Gateway("missing data in metaobjectDefinitions response".into())
        })?;

        for edge in data.metaobject_definitions.edges {
            let access = edge.node.access.map(|a| MetaobjectAccessConfig {
                admin: a.admin,
                storefront: a.storefront,
            });

            let capabilities = edge
                .node
                .capabilities
                .map(|c| MetaobjectCapabilitiesConfig {
                    publishable: c.publishable.map(|f| CapabilityFlag { enabled: f.enabled }),
                    translatable: c
                        .translatable
                        .map(|f| CapabilityFlag { enabled: f.enabled }),
                    renderable: c.renderable.map(|f| CapabilityFlag { enabled: f.enabled }),
                    online_store: c
                        .online_store
                        .map(|f| CapabilityFlag { enabled: f.enabled }),
                });

            let field_definitions = edge
                .node
                .field_definitions
                .into_iter()
                .map(|f| ShopifyMetaobjectFieldDefinition {
                    key: f.key,
                    name: f.name,
                    type_name: f.type_obj.name,
                    required: f.required,
                    description: f.description,
                    validations: f
                        .validations
                        .into_iter()
                        .map(|v| MetaobjectValidationRule {
                            name: v.name,
                            value: v.value,
                        })
                        .collect(),
                })
                .collect();

            out.push(ShopifyMetaobjectDefinition {
                id: Some(edge.node.id),
                type_name: edge.node.r#type,
                name: edge.node.name,
                description: edge.node.description,
                display_name_key: edge.node.display_name_key,
                access,
                capabilities,
                field_definitions,
            });
        }

        if data.metaobject_definitions.page_info.has_next_page {
            after = data.metaobject_definitions.page_info.end_cursor;
            if after.is_none() {
                break;
            }
        } else {
            break;
        }
    }

    Ok(out)
}

pub fn fetch_metaobject_type_to_id_map(
    client: &ShopifyClient,
    logger: &dyn Logger,
) -> Result<HashMap<String, String>, AppError> {
    let id_to_type = fetch_metaobject_id_to_type_map(client, logger)?;
    let mut out: HashMap<String, String> = HashMap::new();
    for (id, t) in id_to_type {
        out.insert(t, id);
    }
    Ok(out)
}

pub fn metaobject_definition_create(
    client: &ShopifyClient,
    input: &mds_app::metaobjects::import::MetaobjectDefinitionCreateInput,
    logger: &dyn Logger,
) -> Result<(), AppError> {
    logger.log(
        LogLevel::Debug,
        "GraphQL metaobjectDefinitionCreate",
        &[
            LogField::new("type", input.type_name.clone()),
            LogField::new("name", input.name.clone()),
        ],
    );

    let vars = MetaobjectDefinitionCreateVars { definition: input };
    let body = client.post_graphql::<_, MetaobjectDefinitionCreateData>(
        graphql::METAOBJECT_DEFINITION_CREATE_MUTATION,
        vars,
        logger,
    )?;

    if let Some(errs) = body.errors {
        return Err(AppError::Gateway(format!(
            "GraphQL errors in metaobjectDefinitionCreate: {errs}"
        )));
    }
    let data = body
        .data
        .ok_or_else(|| AppError::Gateway("missing data in metaobjectDefinitionCreate".into()))?;

    if !data.metaobject_definition_create.user_errors.is_empty() {
        return Err(AppError::Gateway(format!(
            "metaobjectDefinitionCreate userErrors: {}",
            format_user_errors(&data.metaobject_definition_create.user_errors)
        )));
    }

    if data
        .metaobject_definition_create
        .metaobject_definition
        .is_none()
    {
        return Err(AppError::Gateway(
            "metaobjectDefinitionCreate did not return metaobjectDefinition".into(),
        ));
    }

    Ok(())
}

pub fn metaobject_definition_update(
    client: &ShopifyClient,
    id: &str,
    input: &mds_app::metaobjects::import::MetaobjectDefinitionUpdateInput,
    logger: &dyn Logger,
) -> Result<(), AppError> {
    logger.log(
        LogLevel::Debug,
        "GraphQL metaobjectDefinitionUpdate",
        &[LogField::new("id", id.to_string())],
    );

    let vars = MetaobjectDefinitionUpdateVars {
        id,
        definition: input,
    };
    let body = client.post_graphql::<_, MetaobjectDefinitionUpdateData>(
        graphql::METAOBJECT_DEFINITION_UPDATE_MUTATION,
        vars,
        logger,
    )?;

    if let Some(errs) = body.errors {
        return Err(AppError::Gateway(format!(
            "GraphQL errors in metaobjectDefinitionUpdate: {errs}"
        )));
    }
    let data = body
        .data
        .ok_or_else(|| AppError::Gateway("missing data in metaobjectDefinitionUpdate".into()))?;

    if !data.metaobject_definition_update.user_errors.is_empty() {
        return Err(AppError::Gateway(format!(
            "metaobjectDefinitionUpdate userErrors: {}",
            format_user_errors(&data.metaobject_definition_update.user_errors)
        )));
    }

    if data
        .metaobject_definition_update
        .metaobject_definition
        .is_none()
    {
        return Err(AppError::Gateway(
            "metaobjectDefinitionUpdate did not return metaobjectDefinition".into(),
        ));
    }

    Ok(())
}
