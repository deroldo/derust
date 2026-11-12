//! Compatibility suite for `sduix::stacx::v1`: every component is built with the typed API
//! and compared with a JSON fixture; the same fixtures are parsed by the real Stac in
//! `tools/stac-compat/v1` (CI job `stac-compat-v1`).
#![cfg(feature = "sdui_stac_1")]

use serde_json::Value;

mod check_test;
mod contract_examples_test;
mod core_test;
mod custom_test;
mod example_reactive_test;
mod example_test;
mod expr_test;
mod fixtures_test;
mod generated_actions;
mod generated_foundation;
mod generated_theme;
mod generated_widgets;
mod inventory;
mod inventory_state;
mod reactive_actions;
mod reactive_ergonomics;
mod reactive_expressions;
mod reactive_fixtures_test;
mod reactive_lists;
mod reactive_network;

/// Directory holding the JSON fixtures (shared with the Dart harness).
pub const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sdui_stac_v1/fixtures");

/// Converts a JSON object into the map type used by the model.
pub fn json_map(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected a JSON object, got {other}"),
    }
}

/// Converts a JSON object into the `BTreeMap<String, Data>` used by the reactive model.
pub fn data_map(
    value: Value,
) -> std::collections::BTreeMap<String, derust::sduix::stacx::v1::state::Data> {
    json_map(value)
        .into_iter()
        .map(|(key, value)| (key, value.into()))
        .collect()
}

/// Equality that treats `8` and `8.0` as the same number (Dart `double`s print either way).
pub fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|other| json_eq(v, other)))
        }
        _ => a == b,
    }
}

/// Loads a fixture by id (`widgets/column`).
pub fn load_fixture(id: &str) -> Value {
    let path = format!("{FIXTURES_DIR}/{id}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("fixture {path} is not JSON: {e}"))
}

/// Directory holding the fixtures of the reactive state vocabulary (fork only; the official
/// fixtures in `fixtures` stay untouched).
pub const REACTIVE_FIXTURES_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/sdui_stac_v1/fixtures_reactive"
);

/// Loads a reactive fixture by id (`actions/set_state`).
pub fn load_reactive_fixture(id: &str) -> Value {
    let path = format!("{REACTIVE_FIXTURES_DIR}/{id}.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("fixture {path} is not JSON: {e}"))
}

/// Authoring aid: with `UPDATE_REACTIVE_FIXTURES` set, a reactive fixture that does not exist yet
/// is written from the value under test. Existing fixtures are never touched; review the new
/// file before committing it (it becomes the expected contract).
pub fn write_missing_reactive_fixture(id: &str, actual: &Value) {
    if std::env::var_os("UPDATE_REACTIVE_FIXTURES").is_none() {
        return;
    }
    let path = format!("{REACTIVE_FIXTURES_DIR}/{id}.json");
    if std::path::Path::new(&path).exists() {
        return;
    }
    if let Some(dir) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(dir).expect("create fixture directory");
    }
    let mut text = serde_json::to_string_pretty(actual).expect("serializes");
    text.push('\n');
    std::fs::write(&path, text).expect("write fixture");
}

/// Serializes `$value` and compares it with the fixture `$id` (`widgets/column`).
macro_rules! assert_fixture {
    ($id:expr, $value:expr) => {{
        let actual = serde_json::to_value(&$value).expect("serialization failed");
        let expected = $crate::load_fixture($id);
        assert!(
            $crate::json_eq(&actual, &expected),
            "fixture `{}` differs\n actual: {}\nexpected: {}",
            $id,
            serde_json::to_string_pretty(&actual).unwrap(),
            serde_json::to_string_pretty(&expected).unwrap()
        );
    }};
}
pub(crate) use assert_fixture;

/// Like `assert_fixture!`, reading `fixtures_reactive` (`actions/set_state`).
macro_rules! assert_reactive_fixture {
    ($id:expr, $value:expr) => {{
        let actual = serde_json::to_value(&$value).expect("serialization failed");
        $crate::write_missing_reactive_fixture($id, &actual);
        let expected = $crate::load_reactive_fixture($id);
        assert!(
            $crate::json_eq(&actual, &expected),
            "reactive fixture `{}` differs\n actual: {}\nexpected: {}",
            $id,
            serde_json::to_string_pretty(&actual).unwrap(),
            serde_json::to_string_pretty(&expected).unwrap()
        );
    }};
}
pub(crate) use assert_reactive_fixture;
