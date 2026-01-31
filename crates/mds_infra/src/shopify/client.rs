//! Shopify GraphQL HTTP client (transport + envelope handling).

use serde::Serialize;

use mds_app::{
    logging::{LogField, LogLevel, Logger},
    AppError,
};

use crate::shopify::{dto::{GraphQlRequest, GraphQlResponse}, graphql};

#[derive(Debug, Clone)]
pub struct ShopifyClient {
    shop_domain: String,
    access_token: String,
}

impl ShopifyClient {
    pub fn new(shop_domain: String, access_token: String) -> Self {
        Self {
            shop_domain,
            access_token,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "https://{}/admin/api/{}/graphql.json",
            self.shop_domain, graphql::API_VERSION
        )
    }

    pub fn post_graphql<V: Serialize, D: for<'de> serde::Deserialize<'de>>(
        &self,
        query: &str,
        variables: V,
        logger: &dyn Logger,
    ) -> Result<GraphQlResponse<D>, AppError> {
        let client = reqwest::blocking::Client::new();
        let url = self.endpoint();

        let req = GraphQlRequest { query, variables };
        logger.log(
            LogLevel::Debug,
            "GraphQL request",
            &[LogField::new(
                "query_name",
                query.lines().next().unwrap_or("").trim(),
            )],
        );

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Shopify-Access-Token", &self.access_token)
            .json(&req)
            .send()
            .map_err(|e| AppError::Gateway(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AppError::Gateway(format!(
                "HTTP status {} when calling GraphQL",
                resp.status()
            )));
        }

        let body: GraphQlResponse<D> = resp
            .json()
            .map_err(|e| AppError::Gateway(e.to_string()))?;
        Ok(body)
    }
}

