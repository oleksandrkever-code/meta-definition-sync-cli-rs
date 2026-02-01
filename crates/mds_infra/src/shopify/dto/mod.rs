//! Shopify GraphQL DTOs (request/response envelope + shared nodes).

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GraphQlRequest<'a, V> {
    pub query: &'a str,
    pub variables: V,
}

#[derive(Debug, Deserialize)]
pub struct GraphQlResponse<D> {
    pub data: Option<D>,
    pub errors: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserErrorNode {
    #[serde(default)]
    pub field: Option<Vec<String>>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct IdNode {
    #[allow(dead_code)]
    pub id: String,
}

pub fn format_user_errors(errors: &[UserErrorNode]) -> String {
    errors
        .iter()
        .map(|e| {
            if let Some(path) = e.field.as_ref() {
                format!("{}: {}", path.join("."), e.message)
            } else {
                e.message.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub mod metafields;
pub mod metaobjects;
