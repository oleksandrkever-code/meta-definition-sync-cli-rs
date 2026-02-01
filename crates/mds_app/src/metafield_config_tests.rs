#[cfg(test)]
mod tests {
    use crate::metafield_config::MetafieldDefinitionConfig;
    use crate::validation::{format_validation_errors, parse_json_with_path};

    #[test]
    fn validation_reports_path_for_invalid_enum_value() {
        // Human explanation:
        // This is the closest Rust equivalent to Zod reporting:
        // if a field has an invalid enum value, we want to show the JSON path
        // like `0.access.storefront` instead of a vague error.
        let json = r#"
[
  {
    "namespace": "custom",
    "key": "bundle_gallery",
    "name": "Bundle Gallery",
    "type": "single_line_text_field",
    "access": {
      "storefront": "WRONG_VALUE"
    }
  }
]
"#;

        let err = parse_json_with_path::<Vec<MetafieldDefinitionConfig>>(json).unwrap_err();
        let msg = format_validation_errors(&err);

        // Print for learning/debugging: serde_path_to_error provides the exact failing path.
        println!("{msg}");

        // Depending on serde's internal error attribution, the path may point at the field
        // or at its parent object. We accept either, but we require at least the array index
        // and "access" to be present.
        assert!(msg.contains("0"));
        assert!(msg.contains("access"));
    }

    #[test]
    fn validation_reports_path_for_unknown_field() {
        // Human explanation:
        // Node used Zod `.strict()` to reject unknown keys. In Rust we do the same
        // with `#[serde(deny_unknown_fields)]`, and we want an error path to point
        // at the offending object (here: the first array element).
        let json = r#"
[
  {
    "namespace": "custom",
    "key": "bundle_gallery",
    "name": "Bundle Gallery",
    "type": "single_line_text_field",
    "unknownField": 123
  }
]
"#;

        let err = parse_json_with_path::<Vec<MetafieldDefinitionConfig>>(json).unwrap_err();
        let msg = format_validation_errors(&err);

        // The exact message can vary, but the path should include the array index.
        assert!(msg.contains("0"));
        assert!(msg.contains("unknown field"));
    }
}
