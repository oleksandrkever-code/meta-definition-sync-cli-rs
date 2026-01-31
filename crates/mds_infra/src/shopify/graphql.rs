//! Shopify GraphQL query/mutation strings (infrastructure boundary).
//!
//! Keep raw GraphQL documents here to keep adapters readable.

pub const API_VERSION: &str = "2025-10";

pub const GET_METAFIELD_DEFINITIONS_QUERY: &str = r#"
query metafieldDefinitions($ownerType: MetafieldOwnerType!, $first: Int!, $after: String) {
  metafieldDefinitions(ownerType: $ownerType, first: $first, after: $after) {
    edges {
      node {
        id
        namespace
        key
        name
        description
        type {
          name
        }
        validations {
          name
          value
        }
        pinnedPosition
        access {
          admin
          storefront
          customerAccount
        }
        capabilities {
          adminFilterable {
            enabled
          }
          smartCollectionCondition {
            enabled
          }
          uniqueValues {
            enabled
          }
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
"#;

pub const GET_METAOBJECT_DEFINITIONS_QUERY: &str = r#"
query metaobjectDefinitions($first: Int!, $after: String) {
  metaobjectDefinitions(first: $first, after: $after) {
    edges {
      node {
        id
        type
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
"#;

pub const METAFIELD_DEFINITION_CREATE_MUTATION: &str = r#"
mutation metafieldDefinitionCreate($definition: MetafieldDefinitionInput!) {
  metafieldDefinitionCreate(definition: $definition) {
    createdDefinition {
      id
    }
    userErrors {
      field
      message
    }
  }
}
"#;

pub const METAFIELD_DEFINITION_UPDATE_MUTATION: &str = r#"
mutation metafieldDefinitionUpdate($id: ID!, $definition: MetafieldDefinitionInput!) {
  metafieldDefinitionUpdate(id: $id, definition: $definition) {
    updatedDefinition {
      id
    }
    userErrors {
      field
      message
    }
  }
}
"#;

pub const METAFIELD_DEFINITION_DELETE_MUTATION: &str = r#"
mutation metafieldDefinitionDelete($id: ID!) {
  metafieldDefinitionDelete(id: $id) {
    deletedDefinitionId
    userErrors {
      field
      message
    }
  }
}
"#;

pub const METAOBJECT_DEFINITION_CREATE_MUTATION: &str = r#"
mutation metaobjectDefinitionCreate($definition: MetaobjectDefinitionCreateInput!) {
  metaobjectDefinitionCreate(definition: $definition) {
    metaobjectDefinition {
      id
    }
    userErrors {
      field
      message
    }
  }
}
"#;

pub const METAOBJECT_DEFINITION_UPDATE_MUTATION: &str = r#"
mutation metaobjectDefinitionUpdate($id: ID!, $definition: MetaobjectDefinitionUpdateInput!) {
  metaobjectDefinitionUpdate(id: $id, definition: $definition) {
    metaobjectDefinition {
      id
    }
    userErrors {
      field
      message
    }
  }
}
"#;

pub const GET_METAOBJECT_DEFINITIONS_EXPORT_QUERY: &str = r#"
query metaobjectDefinitions($first: Int!, $after: String) {
    metaobjectDefinitions(first: $first, after: $after) {
        edges {
            node {
                id
                type
                name
                description
                displayNameKey
                access {
                    admin
                    storefront
                }
                capabilities {
                    publishable {
                        enabled
                    }
                    translatable {
                        enabled
                    }
                    renderable {
                        enabled
                    }
                    onlineStore {
                        enabled
                    }
                }
                fieldDefinitions {
                    key
                    name
                    description
                    type {
                        name
                    }
                    required
                    validations {
                        name
                        value
                    }
                }
            }
        }
        pageInfo {
            hasNextPage
            endCursor
        }
    }
}"#;

