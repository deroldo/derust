//! Compatibility suite for `sduix::stacx::v1`: every component is built with the typed API
//! and compared with a JSON fixture; the same fixtures are parsed by the real Stac in
//! `tools/stac-compat/v1` (CI job `stac-compat-v1`).
#![cfg(feature = "sdui_stac_1")]

use serde_json::Value;

mod core_test;
mod custom_test;
mod example_test;
mod expr_test;
mod fixtures_test;
mod generated_actions;
mod generated_foundation;
mod generated_theme;
mod generated_widgets;
mod inventory;
mod reactive_ergonomics;

/// Directory holding the JSON fixtures (shared with the Dart harness).
pub const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sdui_stac_v1/fixtures");

/// Converts a JSON object into the map type used by the model.
pub fn json_map(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("expected a JSON object, got {other}"),
    }
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
