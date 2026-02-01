//! Metafield-related Shopify operations (infra).

use std::collections::HashMap;

use mds_app::{
    logging::{LogField, LogLevel, Logger},
    AppError, MetafieldDefinitionInput, ShopifyMetafieldDefinition,
};
use mds_domain::OwnerType;

use crate::shopify::{
    client::ShopifyClient,
    dto::{format_user_errors, metafields::*},
    graphql, metaobjects,
};

fn convert_validations_id_to_type(
    validations: Vec<MetafieldValidationNode>,
    metaobject_id_to_type: &HashMap<String, String>,
) -> Vec<mds_app::MetafieldValidation> {
    validations
        .into_iter()
        .map(|v| {
            if v.name == "metaobject_definition_id" {
                if let Some(id) = v.value.as_deref() {
                    if let Some(t) = metaobject_id_to_type.get(id) {
                        return mds_app::MetafieldValidation {
                            name: "metaobject_definition_type".to_string(),
                            value: Some(t.clone()),
                        };
                    }
                }
            }
            mds_app::MetafieldValidation {
                name: v.name,
                value: v.value,
            }
        })
        .collect()
}

pub fn list_metafield_definitions(
    client: &ShopifyClient,
    owner_type: OwnerType,
    logger: &dyn Logger,
) -> Result<Vec<ShopifyMetafieldDefinition>, AppError> {
    // For portability (parity with Node exporter):
    // convert metaobject_definition_id -> metaobject_definition_type using a lookup map.
    let metaobject_id_to_type = metaobjects::fetch_metaobject_id_to_type_map(client, logger)?;

    let mut out = vec![];
    let mut after: Option<String> = None;

    loop {
        let vars = MetafieldDefsVars {
            owner_type: owner_type.as_str(),
            first: 10,
            after: after.clone(),
        };

        logger.log(
            LogLevel::Debug,
            "GraphQL metafieldDefinitions vars",
            &[
                LogField::new("owner_type", vars.owner_type),
                LogField::new("first", vars.first.to_string()),
                LogField::new("after", format!("{:?}", vars.after)),
            ],
        );

        let body = client.post_graphql::<_, MetafieldDefsData>(
            graphql::GET_METAFIELD_DEFINITIONS_QUERY,
            vars,
            logger,
        )?;

        if let Some(errs) = body.errors {
            return Err(AppError::Gateway(format!(
                "GraphQL errors when querying metafieldDefinitions({}): {errs}",
                owner_type.as_str()
            )));
        }

        let data = body.data.ok_or_else(|| {
            AppError::Gateway("missing data in metafieldDefinitions response".into())
        })?;

        for edge in data.metafield_definitions.edges {
            let converted_validations =
                convert_validations_id_to_type(edge.node.validations, &metaobject_id_to_type);

            out.push(ShopifyMetafieldDefinition {
                id: Some(edge.node.id),
                namespace: edge.node.namespace,
                key: edge.node.key,
                name: edge.node.name,
                description: edge.node.description,
                type_name: edge.node.type_obj.name,
                validations: converted_validations,
                pinned_position: edge.node.pinned_position,
                access: edge.node.access.map(|a| mds_app::MetafieldAccess {
                    admin: a.admin,
                    storefront: a.storefront,
                    customer_account: a.customer_account,
                }),
                capabilities: edge
                    .node
                    .capabilities
                    .map(|c| mds_app::MetafieldCapabilities {
                        admin_filterable: c
                            .admin_filterable
                            .map(|x| mds_app::CapabilityFlag { enabled: x.enabled }),
                        smart_collection_condition: c
                            .smart_collection_condition
                            .map(|x| mds_app::CapabilityFlag { enabled: x.enabled }),
                        unique_values: c
                            .unique_values
                            .map(|x| mds_app::CapabilityFlag { enabled: x.enabled }),
                    }),
            });
        }

        if data.metafield_definitions.page_info.has_next_page {
            after = data.metafield_definitions.page_info.end_cursor;
            if after.is_none() {
                // Defensive: avoid infinite loop on inconsistent API response.
                break;
            }
        } else {
            break;
        }
    }

    Ok(out)
}

pub fn metafield_definition_create(
    client: &ShopifyClient,
    input: &MetafieldDefinitionInput,
    logger: &dyn Logger,
) -> Result<(), AppError> {
    logger.log(
        LogLevel::Debug,
        "GraphQL metafieldDefinitionCreate",
        &[
            LogField::new("namespace", input.namespace.clone()),
            LogField::new("key", input.key.clone()),
        ],
    );

    let vars = MetafieldDefinitionCreateVars { definition: input };
    let body = client.post_graphql::<_, MetafieldDefinitionCreateData>(
        graphql::METAFIELD_DEFINITION_CREATE_MUTATION,
        vars,
        logger,
    )?;

    if let Some(errs) = body.errors {
        return Err(AppError::Gateway(format!(
            "GraphQL errors in metafieldDefinitionCreate: {errs}"
        )));
    }
    let data = body
        .data
        .ok_or_else(|| AppError::Gateway("missing data in metafieldDefinitionCreate".into()))?;

    if !data.metafield_definition_create.user_errors.is_empty() {
        return Err(AppError::Gateway(format!(
            "metafieldDefinitionCreate userErrors: {}",
            format_user_errors(&data.metafield_definition_create.user_errors)
        )));
    }

    if data
        .metafield_definition_create
        .created_definition
        .is_none()
    {
        return Err(AppError::Gateway(
            "metafieldDefinitionCreate did not return createdDefinition".into(),
        ));
    }

    Ok(())
}

pub fn metafield_definition_update(
    client: &ShopifyClient,
    id: &str,
    input: &MetafieldDefinitionInput,
    logger: &dyn Logger,
) -> Result<(), AppError> {
    logger.log(
        LogLevel::Debug,
        "GraphQL metafieldDefinitionUpdate",
        &[
            LogField::new("id", id.to_string()),
            LogField::new("namespace", input.namespace.clone()),
            LogField::new("key", input.key.clone()),
        ],
    );

    let vars = MetafieldDefinitionUpdateVars {
        id,
        definition: input,
    };
    let body = client.post_graphql::<_, MetafieldDefinitionUpdateData>(
        graphql::METAFIELD_DEFINITION_UPDATE_MUTATION,
        vars,
        logger,
    )?;

    if let Some(errs) = body.errors {
        return Err(AppError::Gateway(format!(
            "GraphQL errors in metafieldDefinitionUpdate: {errs}"
        )));
    }
    let data = body
        .data
        .ok_or_else(|| AppError::Gateway("missing data in metafieldDefinitionUpdate".into()))?;

    if !data.metafield_definition_update.user_errors.is_empty() {
        return Err(AppError::Gateway(format!(
            "metafieldDefinitionUpdate userErrors: {}",
            format_user_errors(&data.metafield_definition_update.user_errors)
        )));
    }

    if data
        .metafield_definition_update
        .updated_definition
        .is_none()
    {
        return Err(AppError::Gateway(
            "metafieldDefinitionUpdate did not return updatedDefinition".into(),
        ));
    }

    Ok(())
}

pub fn metafield_definition_delete(
    client: &ShopifyClient,
    id: &str,
    logger: &dyn Logger,
) -> Result<(), AppError> {
    logger.log(
        LogLevel::Debug,
        "GraphQL metafieldDefinitionDelete",
        &[LogField::new("id", id.to_string())],
    );

    let vars = MetafieldDefinitionDeleteVars { id };
    let body = client.post_graphql::<_, MetafieldDefinitionDeleteData>(
        graphql::METAFIELD_DEFINITION_DELETE_MUTATION,
        vars,
        logger,
    )?;

    if let Some(errs) = body.errors {
        return Err(AppError::Gateway(format!(
            "GraphQL errors in metafieldDefinitionDelete: {errs}"
        )));
    }
    let data = body
        .data
        .ok_or_else(|| AppError::Gateway("missing data in metafieldDefinitionDelete".into()))?;

    if !data.metafield_definition_delete.user_errors.is_empty() {
        return Err(AppError::Gateway(format!(
            "metafieldDefinitionDelete userErrors: {}",
            format_user_errors(&data.metafield_definition_delete.user_errors)
        )));
    }

    if data
        .metafield_definition_delete
        .deleted_definition_id
        .is_none()
    {
        return Err(AppError::Gateway(
            "metafieldDefinitionDelete did not return deletedDefinitionId".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_metaobject_definition_id_to_type_when_mapping_exists() {
        let mut map = HashMap::new();
        map.insert(
            "gid://shopify/MetaobjectDefinition/123".to_string(),
            "content_blocks".to_string(),
        );

        let input = vec![
            MetafieldValidationNode {
                name: "metaobject_definition_id".to_string(),
                value: Some("gid://shopify/MetaobjectDefinition/123".to_string()),
            },
            MetafieldValidationNode {
                name: "other".to_string(),
                value: Some("x".to_string()),
            },
        ];

        let out = convert_validations_id_to_type(input, &map);
        assert_eq!(out[0].name, "metaobject_definition_type");
        assert_eq!(out[0].value.as_deref(), Some("content_blocks"));
        assert_eq!(out[1].name, "other");
    }
}
