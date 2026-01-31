//! Domain layer (entities + value objects + pure business rules).

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnerType {
    Product,
    ProductVariant,
    Order,
    Page,
    Shop,
    Collection,
    Customer,
    Blog,
    Article,
    Market,
}

impl OwnerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OwnerType::Product => "PRODUCT",
            OwnerType::ProductVariant => "PRODUCTVARIANT",
            OwnerType::Order => "ORDER",
            OwnerType::Page => "PAGE",
            OwnerType::Shop => "SHOP",
            OwnerType::Collection => "COLLECTION",
            OwnerType::Customer => "CUSTOMER",
            OwnerType::Blog => "BLOG",
            OwnerType::Article => "ARTICLE",
            OwnerType::Market => "MARKET",
        }
    }

    pub fn all() -> Vec<OwnerType> {
        vec![
            OwnerType::Product,
            OwnerType::ProductVariant,
            OwnerType::Order,
            OwnerType::Page,
            OwnerType::Shop,
            OwnerType::Collection,
            OwnerType::Customer,
            OwnerType::Blog,
            OwnerType::Article,
            OwnerType::Market,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOwnerTypeError;

impl std::fmt::Display for ParseOwnerTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid owner type")
    }
}

impl std::error::Error for ParseOwnerTypeError {}

impl FromStr for OwnerType {
    type Err = ParseOwnerTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "PRODUCT" => Ok(OwnerType::Product),
            "PRODUCTVARIANT" => Ok(OwnerType::ProductVariant),
            "ORDER" => Ok(OwnerType::Order),
            "PAGE" => Ok(OwnerType::Page),
            "SHOP" => Ok(OwnerType::Shop),
            "COLLECTION" => Ok(OwnerType::Collection),
            "CUSTOMER" => Ok(OwnerType::Customer),
            "BLOG" => Ok(OwnerType::Blog),
            "ARTICLE" => Ok(OwnerType::Article),
            "MARKET" => Ok(OwnerType::Market),
            _ => Err(ParseOwnerTypeError),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOwnerTypesError {
    pub invalid: Vec<String>,
}

impl std::fmt::Display for ParseOwnerTypesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid owner types: {}", self.invalid.join(", "))
    }
}

impl std::error::Error for ParseOwnerTypesError {}

/// Parse CLI `--owner-type` value.
///
/// Supports:
/// - single value: `PRODUCT`
/// - comma-separated: `PRODUCT,COLLECTION`
/// - special value: `ALL`
pub fn parse_owner_types(input: &str) -> Result<Vec<OwnerType>, ParseOwnerTypesError> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("ALL") {
        return Ok(OwnerType::all());
    }

    let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut invalid = vec![];
    let mut out = vec![];

    for p in parts {
        match p.parse::<OwnerType>() {
            Ok(v) => out.push(v),
            Err(_) => invalid.push(p.to_string()),
        }
    }

    if !invalid.is_empty() {
        return Err(ParseOwnerTypesError { invalid });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_type_as_str_matches_shopify_values() {
        assert_eq!(OwnerType::Product.as_str(), "PRODUCT");
        assert_eq!(OwnerType::ProductVariant.as_str(), "PRODUCTVARIANT");
    }

    #[test]
    fn owner_type_parses_case_insensitive() {
        assert_eq!("product".parse::<OwnerType>().unwrap(), OwnerType::Product);
    }

    #[test]
    fn parse_owner_types_all() {
        let all = parse_owner_types("ALL").unwrap();
        assert_eq!(all, OwnerType::all());
    }

    #[test]
    fn parse_owner_types_comma_separated() {
        let types = parse_owner_types("PRODUCT, COLLECTION").unwrap();
        assert_eq!(types, vec![OwnerType::Product, OwnerType::Collection]);
    }

    #[test]
    fn parse_owner_types_reports_invalid() {
        let err = parse_owner_types("PRODUCT,NOPE,COLLECTION").unwrap_err();
        assert_eq!(err.invalid, vec!["NOPE".to_string()]);
    }
}
