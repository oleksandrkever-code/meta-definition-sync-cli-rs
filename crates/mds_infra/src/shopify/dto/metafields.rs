//! GraphQL DTOs for metafieldDefinitions queries and metafieldDefinition mutations.

use serde::{Deserialize, Serialize};

use mds_app::MetafieldDefinitionInput;

use super::{IdNode, PageInfo, UserErrorNode};

// -------------------------------
// Query: metafieldDefinitions
// -------------------------------

#[derive(Debug, Serialize)]
pub struct MetafieldDefsVars<'a> {
    #[serde(rename = "ownerType")]
    pub owner_type: &'a str,
    pub first: i32,
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefsData {
    #[serde(rename = "metafieldDefinitions")]
    pub metafield_definitions: MetafieldDefsConnection,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefsConnection {
    pub edges: Vec<MetafieldDefEdge>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefEdge {
    pub node: MetafieldDefNode,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefNode {
    pub id: String,
    pub namespace: String,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_obj: MetafieldTypeObj,
    #[serde(default)]
    pub validations: Vec<MetafieldValidationNode>,
    #[serde(rename = "pinnedPosition")]
    pub pinned_position: Option<i32>,
    #[serde(default)]
    pub access: Option<MetafieldAccessNode>,
    #[serde(default)]
    pub capabilities: Option<MetafieldCapabilitiesNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldTypeObj {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldValidationNode {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldAccessNode {
    pub admin: Option<String>,
    pub storefront: Option<String>,
    #[serde(rename = "customerAccount")]
    pub customer_account: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CapabilityFlagNode {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldCapabilitiesNode {
    #[serde(rename = "adminFilterable")]
    pub admin_filterable: Option<CapabilityFlagNode>,
    #[serde(rename = "smartCollectionCondition")]
    pub smart_collection_condition: Option<CapabilityFlagNode>,
    #[serde(rename = "uniqueValues")]
    pub unique_values: Option<CapabilityFlagNode>,
}

// -------------------------------
// Mutations
// -------------------------------

#[derive(Debug, Serialize)]
pub struct MetafieldDefinitionCreateVars<'a> {
    pub definition: &'a MetafieldDefinitionInput,
}

#[derive(Debug, Serialize)]
pub struct MetafieldDefinitionUpdateVars<'a> {
    pub id: &'a str,
    pub definition: &'a MetafieldDefinitionInput,
}

#[derive(Debug, Serialize)]
pub struct MetafieldDefinitionDeleteVars<'a> {
    pub id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionCreatePayload {
    #[serde(rename = "createdDefinition")]
    pub created_definition: Option<IdNode>,
    #[serde(rename = "userErrors", default)]
    pub user_errors: Vec<UserErrorNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionUpdatePayload {
    #[serde(rename = "updatedDefinition")]
    pub updated_definition: Option<IdNode>,
    #[serde(rename = "userErrors", default)]
    pub user_errors: Vec<UserErrorNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionDeletePayload {
    #[serde(rename = "deletedDefinitionId")]
    pub deleted_definition_id: Option<String>,
    #[serde(rename = "userErrors", default)]
    pub user_errors: Vec<UserErrorNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionCreateData {
    #[serde(rename = "metafieldDefinitionCreate")]
    pub metafield_definition_create: MetafieldDefinitionCreatePayload,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionUpdateData {
    #[serde(rename = "metafieldDefinitionUpdate")]
    pub metafield_definition_update: MetafieldDefinitionUpdatePayload,
}

#[derive(Debug, Deserialize)]
pub struct MetafieldDefinitionDeleteData {
    #[serde(rename = "metafieldDefinitionDelete")]
    pub metafield_definition_delete: MetafieldDefinitionDeletePayload,
}

