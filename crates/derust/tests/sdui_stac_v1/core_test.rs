//! Behavior of the core of the model: omission of unset fields, nesting, state references and
//! validation (the per-component fixtures live in the `generated_*` modules).
use derust::sduix::stacx::v1::prelude::*;
use serde_json::json;

#[test]
fn unset_optional_fields_are_omitted() {
    let value = serde_json::to_value(Widget::from(column())).unwrap();
    assert_eq!(value, json!({"type": "column"}));
}

#[test]
fn discriminators_match_stac() {
    assert_eq!(Widget::from(text("a")).type_name(), "text");
    assert_eq!(Action::from(navigate()).type_name(), "navigate");
    assert_eq!(Widget::from(column()).type_name(), Column::TYPE);
}

#[test]
fn state_reference_serializes_as_template() {
    let widget = Widget::from(text(StateRef::new("user.name")));
    assert_eq!(
        serde_json::to_value(widget).unwrap(),
        json!({"type": "text", "data": "{{user.name}}"})
    );
}

#[test]
fn nested_structure_is_typed() {
    let screen = Widget::from(
        container()
            .padding(EdgeInsets::all(8))
            .child(column().children([Widget::from(text("a")), Widget::from(text("b"))])),
    );
    let value = serde_json::to_value(&screen).unwrap();
    assert_eq!(value["child"]["children"][1]["data"], "b");
    assert_eq!(value["padding"]["left"], 8.0);
    assert!(screen.validate().is_empty());
}

#[test]
fn deep_trees_serialize_and_validate() {
    let mut widget = Widget::from(text("leaf"));
    for _ in 0..50 {
        widget = Widget::from(container().child(widget));
    }
    let value = serde_json::to_value(&widget).unwrap();
    let mut depth = 0;
    let mut cursor = &value;
    while let Some(child) = cursor.get("child") {
        depth += 1;
        cursor = child;
    }
    assert_eq!(depth, 50);
    assert_eq!(cursor["data"], "leaf");
    assert!(widget.validate().is_empty());
}

#[test]
fn invalid_trees_are_reported_with_paths() {
    // `StacBadge` asserts `count >= 0` and `maxCount > 0` in the Dart model.
    let screen = Widget::from(column().children([Widget::from(badge().count(-1).max_count(0))]));
    let issues = screen.validate();
    let paths: Vec<_> = issues.iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["$.children[0].count", "$.children[0].maxCount"]);

    let screen = Widget::from(container().padding(EdgeInsets::new().left(f64::NAN)));
    assert_eq!(screen.validate()[0].path, "$.padding.left");

    let screen = Widget::from(text(StateRef::new("")));
    assert_eq!(screen.validate().len(), 1);
}

#[test]
fn versions_are_queryable() {
    assert_eq!(derust::sduix::stacx::versions(), [("v1", "1.6.0")]);
}
