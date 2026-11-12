//! Completeness: every supported/partial inventory item has a fixture file and a test using it.
use crate::inventory::items;

const GENERATED: [&str; 4] = [
    include_str!("generated_widgets.rs"),
    include_str!("generated_actions.rs"),
    include_str!("generated_foundation.rs"),
    include_str!("generated_theme.rs"),
];

/// Source with all whitespace removed (rustfmt may wrap the macro arguments).
fn squash(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn every_supported_item_has_fixture_and_test() {
    let mut missing = Vec::new();
    for item in items() {
        if item["status"] == "unsupported" {
            continue;
        }
        let name = item["name"].as_str().unwrap();
        let Some(fixture) = item["fixture"].as_str() else {
            missing.push(format!("{name}: no fixture in inventory"));
            continue;
        };
        let path = format!("{}/{fixture}.json", crate::FIXTURES_DIR);
        if !std::path::Path::new(&path).exists() {
            missing.push(format!("{name}: fixture file {path} not found"));
        }
        let needle = format!("assert_fixture!(\"{fixture}\",");
        if !GENERATED
            .iter()
            .any(|source| squash(source).contains(&needle))
        {
            missing.push(format!("{name}: no test uses fixture {fixture}"));
        }
    }
    assert!(
        missing.is_empty(),
        "incomplete coverage:\n{}",
        missing.join("\n")
    );
}

#[test]
fn no_orphan_fixtures() {
    fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "json") {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
    let mut files = Vec::new();
    walk(std::path::Path::new(crate::FIXTURES_DIR), &mut files);
    let known: std::collections::HashSet<String> = items()
        .iter()
        .filter_map(|i| i["fixture"].as_str())
        .map(|f| format!("{}/{f}.json", crate::FIXTURES_DIR))
        .collect();
    let orphans: Vec<_> = files
        .iter()
        .filter(|f| !known.contains(*f) && !f.contains("/examples/"))
        .collect();
    assert!(
        orphans.is_empty(),
        "fixtures not in the inventory: {orphans:?}"
    );
}
