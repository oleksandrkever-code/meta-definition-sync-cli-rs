//! GraphQL DTOs for metaobjectDefinitions queries and metaobjectDefinition mutations.

use serde::{Deserialize, Serialize};

use mds_app::metaobjects::import::{
    MetaobjectDefinitionCreateInput, MetaobjectDefinitionUpdateInput,
};

use super::{IdNode, PageInfo, UserErrorNode};

#[derive(Debug, Serialize)]
pub struct MetaobjectDefsVars {
    pub first: i32,
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefsData {
    #[serde(rename = "metaobjectDefinitions")]
    pub metaobject_definitions: MetaobjectDefsConnection,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefsConnection {
    pub edges: Vec<MetaobjectDefEdge>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefEdge {
    pub node: MetaobjectDefNode,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefNode {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectFieldTypeNodeForExport {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectValidationNodeForExport {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectFieldDefForExport {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_obj: MetaobjectFieldTypeNodeForExport,
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub validations: Vec<MetaobjectValidationNodeForExport>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectAccessForExport {
    #[serde(default)]
    pub admin: Option<String>,
    #[serde(default)]
    pub storefront: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectCapabilityFlagNodeForExport {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectCapabilitiesNodeForExport {
    #[serde(default)]
    pub publishable: Option<MetaobjectCapabilityFlagNodeForExport>,
    #[serde(default)]
    pub translatable: Option<MetaobjectCapabilityFlagNodeForExport>,
    #[serde(default)]
    pub renderable: Option<MetaobjectCapabilityFlagNodeForExport>,
    #[serde(rename = "onlineStore")]
    #[serde(default)]
    pub online_store: Option<MetaobjectCapabilityFlagNodeForExport>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefNodeForExport {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<MetaobjectFieldDefForExport>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "displayNameKey")]
    #[serde(default)]
    pub display_name_key: Option<String>,
    #[serde(default)]
    pub access: Option<MetaobjectAccessForExport>,
    #[serde(default)]
    pub capabilities: Option<MetaobjectCapabilitiesNodeForExport>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefsDataForExport {
    #[serde(rename = "metaobjectDefinitions")]
    pub metaobject_definitions: MetaobjectDefsConnectionForExport,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefsConnectionForExport {
    pub edges: Vec<MetaobjectDefEdgeForExport>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefEdgeForExport {
    pub node: MetaobjectDefNodeForExport,
}

// -------------------------------
// Mutations
// -------------------------------

#[derive(Debug, Serialize)]
pub struct MetaobjectDefinitionCreateVars<'a> {
    pub definition: &'a MetaobjectDefinitionCreateInput,
}

#[derive(Debug, Serialize)]
pub struct MetaobjectDefinitionUpdateVars<'a> {
    pub id: &'a str,
    pub definition: &'a MetaobjectDefinitionUpdateInput,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefinitionCreatePayload {
    #[serde(rename = "metaobjectDefinition")]
    pub metaobject_definition: Option<IdNode>,
    #[serde(rename = "userErrors", default)]
    pub user_errors: Vec<UserErrorNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefinitionUpdatePayload {
    #[serde(rename = "metaobjectDefinition")]
    pub metaobject_definition: Option<IdNode>,
    #[serde(rename = "userErrors", default)]
    pub user_errors: Vec<UserErrorNode>,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefinitionCreateData {
    #[serde(rename = "metaobjectDefinitionCreate")]
    pub metaobject_definition_create: MetaobjectDefinitionCreatePayload,
}

#[derive(Debug, Deserialize)]
pub struct MetaobjectDefinitionUpdateData {
    #[serde(rename = "metaobjectDefinitionUpdate")]
    pub metaobject_definition_update: MetaobjectDefinitionUpdatePayload,
}
