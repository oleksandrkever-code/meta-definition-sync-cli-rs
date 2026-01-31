//! Metaobject dependency planning (pure, testable logic).
//!
//! For import we need to create/update metaobject definitions in dependency-safe order.
//! Dependencies are expressed via validations:
//! `metaobject_definition_type = "<type>"`.
//!
//! Notes:
//! - This is a *graph* (DAG) problem, but we present it as "levels" for batching.
//! - We treat dependencies that are NOT present in the JSON file as "external".
//!   They don't affect level ordering (same as Node implementation).

use std::collections::{HashMap, HashSet};

use crate::error::AppError;

use super::types::{MetaobjectDefinitionConfig, MetaobjectValidationRule};

pub fn extract_metaobject_deps(def: &MetaobjectDefinitionConfig) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for field in &def.field_definitions {
        let Some(validations) = field.validations.as_ref() else {
            continue;
        };
        for v in validations {
            if v.name != "metaobject_definition_type" {
                continue;
            }
            if let Some(t) = v.value.as_deref() {
                let t = t.trim();
                if !t.is_empty() {
                    out.push(t.to_string());
                }
            }
        }
    }
    // de-dup (stable)
    let mut seen: HashSet<String> = HashSet::new();
    out.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

pub fn build_internal_deps_map(
    defs: &[MetaobjectDefinitionConfig],
) -> (HashMap<String, Vec<String>>, HashSet<String>) {
    let internal_types: HashSet<String> = defs.iter().map(|d| d.type_name.clone()).collect();
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for d in defs {
        let all_deps = extract_metaobject_deps(d);
        let internal = all_deps
            .into_iter()
            .filter(|t| internal_types.contains(t))
            .collect::<Vec<_>>();
        deps.insert(d.type_name.clone(), internal);
    }
    (deps, internal_types)
}

/// Compute import levels bottom-up (leaves first), like Node `MetaobjectDependencyGraph.getLevels()`.
///
/// Edge orientation: A depends on B (A -> B). Leaves have no internal dependencies.
pub fn compute_levels(
    internal_deps: &HashMap<String, Vec<String>>,
) -> Result<Vec<Vec<String>>, AppError> {
    let mut heights: HashMap<String, usize> = HashMap::new();
    let mut visiting: HashSet<String> = HashSet::new();

    fn height_of(
        node: &str,
        internal_deps: &HashMap<String, Vec<String>>,
        heights: &mut HashMap<String, usize>,
        visiting: &mut HashSet<String>,
    ) -> Result<usize, AppError> {
        if let Some(v) = heights.get(node) {
            return Ok(*v);
        }
        if visiting.contains(node) {
            return Err(AppError::Json(format!(
                "cycle detected in metaobject dependency graph at `{node}`"
            )));
        }
        visiting.insert(node.to_string());

        let deps = internal_deps.get(node).cloned().unwrap_or_default();
        let mut value = 0usize;
        if !deps.is_empty() {
            let mut max_child = 0usize;
            for dep in deps {
                let h = height_of(&dep, internal_deps, heights, visiting)?;
                if h > max_child {
                    max_child = h;
                }
            }
            value = max_child + 1;
        }

        visiting.remove(node);
        heights.insert(node.to_string(), value);
        Ok(value)
    }

    // Compute heights for all nodes (stable order by key sort).
    let mut nodes = internal_deps.keys().cloned().collect::<Vec<_>>();
    nodes.sort();
    for n in &nodes {
        height_of(n, internal_deps, &mut heights, &mut visiting)?;
    }

    let mut buckets: HashMap<usize, Vec<String>> = HashMap::new();
    for n in nodes {
        let h = *heights.get(&n).unwrap_or(&0);
        buckets.entry(h).or_default().push(n);
    }

    let mut level_indices = buckets.keys().cloned().collect::<Vec<_>>();
    level_indices.sort();
    Ok(level_indices
        .into_iter()
        .map(|idx| {
            let mut v = buckets.remove(&idx).unwrap_or_default();
            v.sort();
            v
        })
        .collect())
}

fn tree_line(prefix: &str, is_last: bool, label: &str) -> String {
    let branch = if is_last { "└── " } else { "├── " };
    format!("{prefix}{branch}{label}\n")
}

fn child_prefix(prefix: &str, is_last: bool) -> String {
    if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    }
}

/// Render a dependency forest like the Node CLI output:
/// roots with nested dependencies (children are "depends on").
pub fn render_dependency_forest_text(
    internal_deps: &HashMap<String, Vec<String>>,
    external_types: &[String],
    missing_external_types: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("🌳 Metaobject dependency graph\n");

    // Compute roots: nodes with no incoming edges (nobody depends on them).
    let mut has_incoming: HashSet<String> = HashSet::new();
    for (from, deps) in internal_deps {
        let _ = from; // keep orientation explicit: from -> dep
        for dep in deps {
            has_incoming.insert(dep.clone());
        }
    }
    let mut roots = internal_deps
        .keys()
        .filter(|t| !has_incoming.contains(*t))
        .cloned()
        .collect::<Vec<_>>();
    roots.sort();

    fn render_subtree(
        out: &mut String,
        internal_deps: &HashMap<String, Vec<String>>,
        prefix: &str,
        node: &str,
        is_last: bool,
        visiting: &mut HashSet<String>,
    ) {
        // Detect cycles defensively; we already validate elsewhere.
        if visiting.contains(node) {
            out.push_str(&tree_line(prefix, is_last, &format!("{node} (cycle)")));
            return;
        }
        visiting.insert(node.to_string());

        out.push_str(&tree_line(prefix, is_last, node));
        let p = child_prefix(prefix, is_last);

        let mut deps = internal_deps.get(node).cloned().unwrap_or_default();
        deps.sort();
        for (i, dep) in deps.iter().enumerate() {
            let last = i + 1 == deps.len();
            render_subtree(out, internal_deps, &p, dep, last, visiting);
        }

        visiting.remove(node);
    }

    let mut visiting: HashSet<String> = HashSet::new();
    for (i, root) in roots.iter().enumerate() {
        let last = i + 1 == roots.len();
        render_subtree(&mut out, internal_deps, "", root, last, &mut visiting);
    }

    // Optional: append external diagnostics only when present.
    if !external_types.is_empty() {
        out.push('\n');
        out.push_str("🔗 External dependencies (already in Shopify)\n");
        for t in external_types {
            out.push_str(&format!("- {t}\n"));
        }
    }
    if !missing_external_types.is_empty() {
        out.push('\n');
        out.push_str("⚠️ Missing external dependencies (not in JSON and not in Shopify)\n");
        for t in missing_external_types {
            out.push_str(&format!("- {t}\n"));
        }
    }

    out
}

pub fn collect_external_dependency_types(
    defs: &[MetaobjectDefinitionConfig],
    internal_types: &HashSet<String>,
) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for d in defs {
        for dep in extract_metaobject_deps(d) {
            if !internal_types.contains(&dep) {
                out.push(dep);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn normalize_description(d: Option<String>) -> Option<String> {
    let s = d?.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn normalize_display_name_key(k: Option<String>) -> Option<String> {
    let s = k?.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn normalize_validations(v: &Option<Vec<MetaobjectValidationRule>>) -> Vec<MetaobjectValidationRule> {
    match v {
        None => vec![],
        Some(list) => list.clone(),
    }
}

