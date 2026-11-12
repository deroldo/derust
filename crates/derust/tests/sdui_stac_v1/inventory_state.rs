//! Schema and coverage checks for `inventory_state.json`, the hand-written inventory of the
//! reactive vocabulary of the fork (envelope, state actions, templates, expression language).
//!
//! Every non-`unsupported` item must point to a fixture that exists, is used by a test and (for
//! expressions) really contains the item's `sample`; every fixture file must be used by a test.
use serde_json::Value;
use std::collections::HashSet;

pub const INVENTORY_STATE: &str = include_str!("../../src/sduix/stacx/v1/inventory_state.json");

/// Test sources whose `assert_reactive_fixture!("<id>", ..)` calls count as using a fixture.
const SOURCES: [&str; 6] = [
    include_str!("reactive_actions.rs"),
    include_str!("reactive_lists.rs"),
    include_str!("reactive_network.rs"),
    include_str!("reactive_fixtures_test.rs"),
    include_str!("contract_examples_test.rs"),
    include_str!("reactive_ergonomics.rs"),
];

pub fn state_items() -> Vec<Value> {
    let doc: Value = serde_json::from_str(INVENTORY_STATE).expect("inventory_state.json is JSON");
    doc["items"].as_array().expect("items array").clone()
}

/// Source with all whitespace removed (rustfmt may wrap the macro arguments).
fn squash(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

fn used_by_a_test(fixture: &str) -> bool {
    let needle = format!("assert_reactive_fixture!(\"{fixture}\",");
    SOURCES.iter().any(|s| squash(s).contains(&needle))
}

#[test]
fn state_inventory_schema_is_valid() {
    let doc: Value = serde_json::from_str(INVENTORY_STATE).unwrap();
    assert_eq!(doc["fork_ref"], "estado_compartilhado_reativo");
    let valid_kinds = [
        "envelope",
        "action",
        "action_field",
        "widget_field",
        "operator",
        "function",
        "literal",
        "binding",
        "root",
        "example",
        "limitation",
    ];
    let valid_status = ["supported", "partial", "unsupported"];
    let mut names = HashSet::new();
    for item in state_items() {
        let name = item["name"].as_str().expect("name");
        assert!(names.insert(name.to_string()), "duplicate item {name}");
        assert!(
            valid_kinds.contains(&item["kind"].as_str().unwrap_or("")),
            "{name}: kind"
        );
        assert_eq!(item["source"], "fork", "{name}: source");
        assert!(
            valid_status.contains(&item["status"].as_str().unwrap_or("")),
            "{name}: status"
        );
        assert!(
            item["json_key"].as_str().is_some_and(|k| !k.is_empty()),
            "{name}: json_key"
        );
        // every item cites the contract section in the notes
        assert!(
            item["notes"].as_str().is_some_and(|n| !n.is_empty()),
            "{name}: notes"
        );
        if item["status"] == "unsupported" {
            assert!(
                item["fixture"].is_null(),
                "{name}: unsupported has no fixture"
            );
            assert_eq!(
                item["kind"], "limitation",
                "{name}: kind of an unsupported item"
            );
        } else {
            assert!(item["fixture"].is_string(), "{name}: fixture");
            let notes = item["notes"].as_str().unwrap();
            assert!(
                notes.contains(".mdx") || item["kind"] == "envelope",
                "{name}: notes must cite the contract section ({notes})"
            );
        }
        let expression_fixture = item["fixture"]
            .as_str()
            .is_some_and(|f| f.starts_with("expressions/"));
        if expression_fixture {
            assert!(
                item["sample"].is_string(),
                "{name}: expression items need a sample"
            );
        }
    }
}

#[test]
fn every_state_item_has_fixture_and_test() {
    let mut problems = Vec::new();
    for item in state_items() {
        if item["status"] == "unsupported" {
            continue;
        }
        let name = item["name"].as_str().unwrap();
        let fixture = item["fixture"].as_str().unwrap();
        let file = format!("{}/{fixture}.json", crate::REACTIVE_FIXTURES_DIR);
        let Ok(text) = std::fs::read_to_string(&file) else {
            problems.push(format!("{name}: fixture file {file} not found"));
            continue;
        };
        if let Some(category) = fixture.strip_prefix("expressions/") {
            // expression fixtures are produced by `reactive_expressions::categories`
            if !crate::reactive_expressions::categories()
                .iter()
                .any(|(c, _)| *c == category)
            {
                problems.push(format!("{name}: no expression category `{category}`"));
            }
            let sample = item["sample"].as_str().unwrap_or("");
            let list: Value = serde_json::from_str(&text).expect("expression fixture is JSON");
            let found = list
                .as_array()
                .is_some_and(|l| l.iter().any(|e| e["source"] == sample));
            if !found {
                problems.push(format!("{name}: sample `{sample}` is not in {fixture}"));
            }
        } else if !used_by_a_test(fixture) {
            problems.push(format!("{name}: no test uses fixture {fixture}"));
        }
    }
    assert!(
        problems.is_empty(),
        "incomplete coverage:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_reactive_fixture_file_is_used_by_a_test() {
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
    walk(
        std::path::Path::new(crate::REACTIVE_FIXTURES_DIR),
        &mut files,
    );
    assert!(files.len() > 100, "found only {} fixtures", files.len());
    let prefix = format!("{}/", crate::REACTIVE_FIXTURES_DIR);
    let orphans: Vec<_> = files
        .iter()
        .map(|f| {
            f.strip_prefix(&prefix)
                .unwrap()
                .trim_end_matches(".json")
                .to_string()
        })
        // expression files are checked against `categories()` by `no_orphan_expression_files`
        .filter(|id| !id.starts_with("expressions/") && !used_by_a_test(id))
        .collect();
    assert!(orphans.is_empty(), "fixtures no test uses: {orphans:?}");
}

#[test]
fn state_inventory_covers_every_expression_category() {
    let used: HashSet<String> = state_items()
        .iter()
        .filter_map(|i| i["fixture"].as_str())
        .filter_map(|f| f.strip_prefix("expressions/"))
        .map(str::to_string)
        .collect();
    for (category, _) in crate::reactive_expressions::categories() {
        assert!(
            used.contains(category),
            "expression category `{category}` has no inventory item"
        );
    }
}

#[test]
fn state_inventory_covers_every_operator_and_function() {
    let samples: Vec<String> = state_items()
        .iter()
        .filter_map(|i| i["sample"].as_str())
        .map(str::to_string)
        .collect();
    let has = |needle: &str| samples.iter().any(|s| s.contains(needle));
    for symbol in [
        " + ", " - ", " * ", " / ", " % ", " < ", " <= ", " > ", " >= ", " == ", " != ", " && ",
        " || ", " ? ", "!", "-", "(", ".", "[",
    ] {
        assert!(has(symbol), "no inventory sample uses `{symbol}`");
    }
    for function in ["length(", "trim(", "contains("] {
        assert!(has(function), "no inventory sample uses `{function}`");
    }
}
