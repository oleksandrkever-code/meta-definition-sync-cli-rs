//! Metaobject-related application DTOs (JSON contract).
//!
//! These structs match the shape of `definitions/metaobjects.json` (Node parity).

use serde::{Deserialize, Serialize};

use crate::metafields::types::CapabilityFlag;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaobjectDefinitionConfig {
    /// Metaobject type (stable identifier).
    #[serde(rename = "type")]
    pub type_name: String,
    pub name: String,
    #[serde(rename = "fieldDefinitions")]
    pub field_definitions: Vec<MetaobjectFieldDefinitionConfig>,

    /// Optional description (sometimes stored as empty string in exports).
    #[serde(default)]
    pub description: Option<String>,

    #[serde(rename = "displayNameKey", default)]
    pub display_name_key: Option<String>,

    #[serde(default)]
    pub access: Option<MetaobjectAccessConfig>,

    #[serde(default)]
    pub capabilities: Option<MetaobjectCapabilitiesConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaobjectFieldDefinitionConfig {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,

    /// Optional description (sometimes stored as empty string in exports).
    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub validations: Option<Vec<MetaobjectValidationRule>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaobjectValidationRule {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaobjectAccessConfig {
    #[serde(default)]
    pub admin: Option<String>,
    #[serde(default)]
    pub storefront: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaobjectCapabilitiesConfig {
    #[serde(default)]
    pub publishable: Option<CapabilityFlag>,
    #[serde(default)]
    pub translatable: Option<CapabilityFlag>,
    #[serde(default)]
    pub renderable: Option<CapabilityFlag>,
    #[serde(rename = "onlineStore", default)]
    pub online_store: Option<CapabilityFlag>,
}
