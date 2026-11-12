//! Schema and coverage checks for `inventory.json`.
use serde_json::Value;
use std::collections::HashSet;

pub const INVENTORY: &str = include_str!("../../src/sduix/stacx/v1/inventory.json");

pub fn items() -> Vec<Value> {
    let doc: Value = serde_json::from_str(INVENTORY).expect("inventory.json is valid JSON");
    doc["items"].as_array().expect("items array").clone()
}

#[test]
fn inventory_matches_pinned_tag() {
    let doc: Value = serde_json::from_str(INVENTORY).unwrap();
    assert_eq!(doc["stac_tag"], "v1.6.0");
    assert_eq!(
        doc["stac_commit"],
        derust::sduix::stacx::v1::STAC_TAG_COMMIT
    );
}

#[test]
fn inventory_schema_is_valid() {
    let valid_categories = ["widget", "action", "foundation", "theme"];
    let valid_status = ["supported", "partial", "unsupported"];
    let mut names = HashSet::new();
    for item in items() {
        let name = item["name"].as_str().expect("name");
        assert!(names.insert(name.to_string()), "duplicate item {name}");
        assert!(
            valid_categories.contains(&item["category"].as_str().unwrap()),
            "{name}: category"
        );
        assert!(
            valid_status.contains(&item["status"].as_str().unwrap()),
            "{name}: status"
        );
        assert!(
            item["dart_path"]
                .as_str()
                .is_some_and(|p| p.ends_with(".dart")),
            "{name}: dart_path"
        );
        assert!(
            item["batch"].as_str().is_some_and(|b| !b.is_empty()),
            "{name}: batch"
        );
        assert!(item.get("json_type").is_some(), "{name}: json_type");
        assert!(item.get("fixture").is_some(), "{name}: fixture");
        if item["status"] == "unsupported" {
            assert!(
                item["notes"].as_str().is_some_and(|n| !n.is_empty()),
                "{name}: unsupported needs notes"
            );
        }
    }
}

#[test]
fn inventory_has_minimum_coverage() {
    let items = items();
    let count = |category: &str| items.iter().filter(|i| i["category"] == category).count();
    assert!(count("widget") >= 90, "widgets: {}", count("widget"));
    assert!(count("action") >= 10, "actions: {}", count("action"));
    assert!(items.iter().any(|i| i["name"] == "StacTheme"));
}
