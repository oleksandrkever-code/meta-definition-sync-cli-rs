//! Metafield-related application DTOs shared across use-cases and ports.

use serde::{Deserialize, Serialize};

/// Minimal shape of metafield definition coming from Shopify (read-model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopifyMetafieldDefinition {
    /// Shopify GID (needed for update/delete on import).
    ///
    /// Present for real Shopify gateway; optional so tests can build this struct without IDs.
    #[serde(default)]
    pub id: Option<String>,
    pub namespace: String,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Shopify metafield type name (e.g. "single_line_text_field")
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub validations: Vec<MetafieldValidation>,
    /// Raw pinnedPosition from API; export converts to `pin: bool`
    #[serde(default)]
    pub pinned_position: Option<i32>,
    #[serde(default)]
    pub access: Option<MetafieldAccess>,
    #[serde(default)]
    pub capabilities: Option<MetafieldCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldValidation {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldAccess {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storefront: Option<String>,
    #[serde(rename = "customerAccount", skip_serializing_if = "Option::is_none")]
    pub customer_account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFlag {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldCapabilities {
    #[serde(rename = "adminFilterable", skip_serializing_if = "Option::is_none")]
    pub admin_filterable: Option<CapabilityFlag>,
    #[serde(
        rename = "smartCollectionCondition",
        skip_serializing_if = "Option::is_none"
    )]
    pub smart_collection_condition: Option<CapabilityFlag>,
    #[serde(rename = "uniqueValues", skip_serializing_if = "Option::is_none")]
    pub unique_values: Option<CapabilityFlag>,
}

// -------------------------------------------------------------------------------------------------
// Import mutation input DTOs (used by ports + infra).
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldDefinitionValidationInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldDefinitionCapabilitiesInput {
    #[serde(rename = "adminFilterable", skip_serializing_if = "Option::is_none")]
    pub admin_filterable: Option<CapabilityFlag>,
    #[serde(
        rename = "smartCollectionCondition",
        skip_serializing_if = "Option::is_none"
    )]
    pub smart_collection_condition: Option<CapabilityFlag>,
    #[serde(rename = "uniqueValues", skip_serializing_if = "Option::is_none")]
    pub unique_values: Option<CapabilityFlag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetafieldDefinitionInput {
    pub namespace: String,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(rename = "ownerType")]
    pub owner_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<MetafieldDefinitionValidationInput>>,
    /// Pin definition in Shopify admin UI.
    ///
    /// NOTE: We intentionally send `pin` (boolean), NOT `pinnedPosition`.
    /// `pinnedPosition` is returned by the API but is not accepted on MetafieldDefinitionInput
    /// (as observed in real error reports).
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub pin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<MetafieldDefinitionCapabilitiesInput>,
    // NOTE: access intentionally excluded (parity with Node "access-free imports").
}

fn is_false(v: &bool) -> bool {
    !*v
}

