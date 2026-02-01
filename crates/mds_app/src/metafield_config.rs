//! JSON config models for metafield definitions (import side).
//!
//! This is the Rust equivalent of the Zod schema validation in the Node CLI,
//! focused on producing good error paths via `serde_path_to_error`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetafieldDefinitionConfig {
    pub namespace: String,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub validations: Option<Vec<ValidationRule>>,
    #[serde(default)]
    pub pin: bool,
    #[serde(default)]
    pub access: Option<MetafieldAccessConfig>,
    #[serde(default)]
    pub capabilities: Option<MetafieldCapabilitiesConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationRule {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetafieldAccessConfig {
    #[serde(default)]
    pub storefront: Option<StorefrontAccess>,
    #[serde(default)]
    pub admin: Option<AdminAccess>,
    #[serde(rename = "customerAccount", default)]
    pub customer_account: Option<CustomerAccountAccess>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorefrontAccess {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "PUBLIC_READ")]
    PublicRead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdminAccess {
    #[serde(rename = "MERCHANT_READ")]
    MerchantRead,
    #[serde(rename = "MERCHANT_READ_WRITE")]
    MerchantReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomerAccountAccess {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "READ")]
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityFlagConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetafieldCapabilitiesConfig {
    #[serde(rename = "adminFilterable", default)]
    pub admin_filterable: Option<CapabilityFlagConfig>,
    #[serde(rename = "smartCollectionCondition", default)]
    pub smart_collection_condition: Option<CapabilityFlagConfig>,
    #[serde(rename = "uniqueValues", default)]
    pub unique_values: Option<CapabilityFlagConfig>,
}
