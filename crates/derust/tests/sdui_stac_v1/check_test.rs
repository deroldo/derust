//! Contextual checks over the serialized screen (`check_screen`): one test per rule, positive and
//! negative.

use derust::sduix::stacx::common::{Validate, ValidationReport};
use derust::sduix::stacx::v1::prelude::*;
use derust::sduix::stacx::v1::state::{check_screen, MAX_CHECK_DEPTH};
use serde_json::{json, Value};

fn errors(report: &ValidationReport) -> Vec<String> {
    report.errors.iter().map(|i| i.to_string()).collect()
}

fn warnings(report: &ValidationReport) -> Vec<String> {
    report.warnings.iter().map(|i| i.to_string()).collect()
}

/// Wraps `children` in a widget that declares `state` (so the scope is `Known`).
fn screen(state: Value, children: Value) -> Value {
    json!({"type": "column", "state": state, "children": children})
}

fn clean(value: Value) {
    let report = check_screen(&value);
    assert_eq!(errors(&report), Vec::<String>::new(), "errors in {value}");
    assert_eq!(
        warnings(&report),
        Vec::<String>::new(),
        "warnings in {value}"
    );
}

fn only_error(value: Value, path: &str, fragment: &str) {
    let report = check_screen(&value);
    let errs = errors(&report);
    assert_eq!(errs.len(), 1, "expected one error, got {errs:?}");
    assert!(errs[0].starts_with(path), "path: {errs:?}");
    assert!(errs[0].contains(fragment), "message: {errs:?}");
}

fn no_error(value: Value) {
    let report = check_screen(&value);
    assert!(report.is_ok(), "unexpected errors {:?}", errors(&report));
}

fn set(key: &str) -> Value {
    json!({"actionType": "setState", "values": {key: 1}})
}

// ---------------------------------------------------------------- contract screens

#[test]
fn contract_examples_have_no_issues() {
    // globalState + onInit
    clean(json!({
        "type": "scaffold",
        "globalState": {"token": {"initial": "", "persist": true, "secure": true}, "theme": "light"},
        "onInit": {"actionType": "setState", "values": {"global.theme": "dark"}},
        "body": {"type": "text", "data": {"$state": "global.theme"}}
    }));
    // form with bind
    clean(screen(
        json!({"name": "", "agree": false, "plan": "free"}),
        json!([
            {"type": "textField", "bind": "name"},
            {"type": "checkBox", "bind": "agree"},
            {"type": "radioGroup", "bind": "plan", "child": {"type": "radio", "value": "free"}},
            {"type": "text", "data": {"$expr": "trim(name) + '!'"}, "visible": {"$expr": "agree && length(name) > 0"}}
        ]),
    ));
    // multiAction
    clean(screen(
        json!({"count": 0, "open": false}),
        json!([{"type": "elevatedButton", "onPressed": {
        "actionType": "multiAction", "sync": true,
        "actions": [
            {"actionType": "incrementState", "key": "count", "by": 2, "max": 10},
            {"actionType": "toggleState", "key": "open", "condition": {"$expr": "count < 10"}}
        ]}}]),
    ));
    // networkRequest with callbacks
    clean(screen(
        json!({"loading": false, "user": null}),
        json!([{"type": "elevatedButton", "onPressed": {
            "actionType": "networkRequest", "url": "https://x", "method": "get", "requestKey": "r",
            "onStart": {"actionType": "setState", "values": {"loading": true}},
            "onSuccess": [{"actionType": "setState", "values": {"user": {"$state": "response.data"}}}],
            "onError": {"actionType": "setState", "values": {"loading": {"$expr": "response.status == 500"}}},
            "onFinally": {"actionType": "setState", "values": {"loading": false}}}}]),
    ));
    // navigate with params / result
    clean(screen(
        json!({"picked": null}),
        json!([{"type": "elevatedButton", "onPressed": {
            "actionType": "navigate", "routeName": "/pick", "params": {"who": {"$state": "picked"}},
            "onResult": {"actionType": "setState", "values": {"picked": {"$state": "result.id"}}},
            "onCancel": {"actionType": "none"}}}]),
    ));
    // list with template
    clean(screen(
        json!({"todos": [], "text": ""}),
        json!([{"type": "listView", "items": {"$state": "todos"}, "as": "todo", "indexAs": "i",
        "itemTemplate": {"type": "row", "children": [
            {"type": "text", "data": {"$expr": "(i + 1) + '. ' + todo.name"}},
            {"type": "iconButton", "onPressed": {"actionType": "removeFromState", "key": "todos", "index": {"$state": "i"}}}
        ]}}]),
    ));
}

#[test]
fn every_fixture_passes_without_errors() {
    fn visit(dir: &std::path::Path, count: &mut usize) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, count);
            } else if path.extension().is_some_and(|e| e == "json") {
                let text = std::fs::read_to_string(&path).unwrap();
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    if value.is_object() {
                        let report = check_screen(&value);
                        assert!(report.is_ok(), "{}: {:?}", path.display(), errors(&report));
                        *count += 1;
                    }
                }
            }
        }
    }
    let mut count = 0;
    visit(std::path::Path::new(crate::FIXTURES_DIR), &mut count);
    visit(
        std::path::Path::new(crate::REACTIVE_FIXTURES_DIR),
        &mut count,
    );
    assert!(count > 100, "only {count} fixtures visited");
}

// ---------------------------------------------------------------- state declaration

#[test]
fn reserved_state_keys() {
    for name in ["global", "params", "result", "response", "event"] {
        only_error(
            screen(json!({ name: 1 }), json!([])),
            &format!("$.state.{name}"),
            "reserved name",
        );
    }
    no_error(screen(json!({"item": 1, "index": 2}), json!([])));
}

#[test]
fn persist_and_secure_only_in_global_state() {
    only_error(
        screen(json!({"k": {"initial": 1, "persist": true}}), json!([])),
        "$.state.k",
        "persist/secure",
    );
    only_error(
        screen(json!({"k": {"secure": true}}), json!([])),
        "$.state.k",
        "persist/secure",
    );
    // a data object that merely has an `initial` key is a value
    no_error(screen(json!({"k": {"initial": 1}}), json!([])));
    no_error(screen(json!({"k": {"initial": 1, "other": 2}}), json!([])));
}

#[test]
fn secure_global_key_cannot_be_non_persistent() {
    only_error(
        json!({"type": "text", "globalState": {"k": {"initial": 1, "secure": true, "persist": false}}}),
        "$.globalState.k",
        "secure",
    );
    no_error(
        json!({"type": "text", "globalState": {"k": {"initial": 1, "secure": true, "persist": true}}}),
    );
    no_error(
        json!({"type": "text", "globalState": {"k": {"initial": 1, "secure": false, "persist": false}}}),
    );
}

#[test]
fn state_blocks_must_be_objects() {
    only_error(json!({"type": "text", "state": [1]}), "$.state", "object");
    only_error(
        json!({"type": "text", "globalState": "x"}),
        "$.globalState",
        "object",
    );
}

// ---------------------------------------------------------------- bind

#[test]
fn bind_requires_a_supported_widget() {
    for (widget, property) in [
        ("checkBox", "value"),
        ("switch", "value"),
        ("slider", "value"),
        ("radioGroup", "groupValue"),
        ("textField", "initialValue"),
        ("textFormField", "initialValue"),
        ("dropdownMenu", "initialSelection"),
    ] {
        no_error(screen(
            json!({"k": 1}),
            json!([{"type": widget, "bind": "k"}]),
        ));
        only_error(
            screen(
                json!({"k": 1}),
                json!([{"type": widget, "bind": "k", property: 1}]),
            ),
            "$.children[0].bind",
            property,
        );
    }
    only_error(
        screen(json!({"k": 1}), json!([{"type": "text", "bind": "k"}])),
        "$.children[0].bind",
        "does not support bind",
    );
}

#[test]
fn bind_on_radio_has_its_own_message() {
    only_error(
        screen(json!({"k": 1}), json!([{"type": "radio", "bind": "k"}])),
        "$.children[0].bind",
        "radioGroup",
    );
}

#[test]
fn bind_must_be_non_empty_text() {
    for bad in [
        json!(""),
        json!("  "),
        json!(3),
        json!(null),
        json!({"a": 1}),
    ] {
        only_error(
            screen(json!({"k": 1}), json!([{"type": "checkBox", "bind": bad}])),
            "$.children[0].bind",
            "non-empty text",
        );
    }
    no_error(screen(
        json!({}),
        json!([{"type": "checkBox", "bind": "global.flag"}]),
    ));
}

// ---------------------------------------------------------------- writes

#[test]
fn writes_to_undeclared_keys_are_errors() {
    let write_actions = [
        json!({"actionType": "incrementState", "key": "n"}),
        json!({"actionType": "decrementState", "key": "n"}),
        json!({"actionType": "toggleState", "key": "n"}),
        json!({"actionType": "appendState", "key": "n", "value": 1}),
        json!({"actionType": "removeFromState", "key": "n", "index": 0}),
        json!({"actionType": "toggleItem", "key": "n", "value": 1}),
    ];
    for action in write_actions {
        let declared = screen(
            json!({"n": 1}),
            json!([{"type": "elevatedButton", "onPressed": action.clone()}]),
        );
        clean(declared);
        only_error(
            screen(
                json!({"m": 1}),
                json!([{"type": "elevatedButton", "onPressed": action}]),
            ),
            "$.children[0].onPressed.key",
            "undeclared state key `n`",
        );
    }
    only_error(
        screen(
            json!({"m": 1}),
            json!([{"type": "elevatedButton", "onPressed": set("n")}]),
        ),
        "$.children[0].onPressed.values.n",
        "undeclared state key `n`",
    );
    clean(screen(
        json!({"n": 1}),
        json!([{"type": "elevatedButton", "onPressed": set("n")}]),
    ));
}

#[test]
fn writes_to_reserved_names_are_errors() {
    for name in ["global", "params", "result", "response", "event"] {
        only_error(
            screen(
                json!({"m": 1}),
                json!([{"type": "elevatedButton", "onPressed": set(name)}]),
            ),
            "$.children[0].onPressed.values",
            &format!("reserved name `{name}`"),
        );
    }
}

#[test]
fn writes_to_global_keys_are_never_reported() {
    clean(screen(
        json!({"m": 1}),
        json!([{"type": "elevatedButton", "onPressed": {"actionType": "setState", "values": {"global.anything": 1}}}]),
    ));
    clean(screen(
        json!({"m": 1}),
        json!([{"type": "elevatedButton", "onPressed": {"actionType": "toggleState", "key": "global.flag"}}]),
    ));
}

#[test]
fn reset_state_ignores_undeclared_keys() {
    clean(screen(
        json!({"m": 1}),
        json!([{"type": "elevatedButton", "onPressed": {"actionType": "resetState", "keys": ["nope"]}}]),
    ));
}

#[test]
fn writes_in_opaque_scopes_are_not_reported() {
    // root without state is opaque
    no_error(json!({"type": "elevatedButton", "onPressed": set("anything")}));
    // navigation targets and overlays are opaque even inside a known scope
    let navigate = json!({"actionType": "navigate", "widgetJson": {"type": "text", "data": {"$state": "zzz"}, "onTap": set("zzz")}});
    clean(screen(
        json!({"m": 1}),
        json!([{"type": "elevatedButton", "onPressed": navigate}]),
    ));
    for action in [
        json!({"actionType": "showDialog", "widget": {"type": "text", "data": {"$expr": "zzz"}, "onTap": set("zzz")}}),
        json!({"actionType": "showModalBottomSheet", "widget": {"type": "text", "onTap": set("zzz")}}),
        json!({"actionType": "showSnackBar", "content": {"type": "text", "data": {"$state": "zzz"}},
            "action": {"label": "x", "onPressed": set("zzz")}}),
    ] {
        clean(screen(
            json!({"m": 1}),
            json!([{"type": "elevatedButton", "onPressed": action}]),
        ));
    }
    // an overlay that declares state becomes known again
    only_error(
        screen(
            json!({"m": 1}),
            json!([{"type": "elevatedButton", "onPressed": {"actionType": "showDialog",
                "widget": {"type": "column", "state": {"a": 1}, "onTap": set("zzz")}}}]),
        ),
        "$.children[0].onPressed.widget.onTap.values.zzz",
        "undeclared",
    );
}

#[test]
fn nested_state_replaces_the_outer_scope() {
    // writing an outer key inside the inner state is an error (no chaining)
    only_error(
        screen(
            json!({"outer": 1}),
            json!([{"type": "column", "state": {"inner": 1}, "children": [
                {"type": "elevatedButton", "onPressed": set("outer")}]}]),
        ),
        "$.children[0].children[0].onPressed.values.outer",
        "undeclared state key `outer`",
    );
    clean(screen(
        json!({"outer": 1}),
        json!([{"type": "column", "state": {"inner": 1}, "children": [
            {"type": "elevatedButton", "onPressed": set("inner")}]}]),
    ));
}

// ---------------------------------------------------------------- variables

#[test]
fn response_scope() {
    let request = |callback: &str| {
        screen(
            json!({"x": 1}),
            json!([{"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": "u",
                callback: {"actionType": "setState", "values": {"x": {"$state": "response.data"}}}}}]),
        )
    };
    for ok in ["onSuccess", "onError", "onFinally"] {
        clean(request(ok));
    }
    only_error(
        request("onStart"),
        "$.children[0].onPressed.onStart.values.x",
        "`response`",
    );
    only_error(
        screen(
            json!({"x": 1}),
            json!([{"type": "text", "data": {"$expr": "response.a"}}]),
        ),
        "$.children[0].data",
        "`response`",
    );
}

#[test]
fn response_flows_into_chained_actions() {
    clean(screen(
        json!({"x": 1}),
        json!([{"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": "u",
            "onSuccess": {"actionType": "multiAction", "actions": [
                {"actionType": "setState", "values": {"x": {"$state": "response.data"}}}]}}}]),
    ));
}

#[test]
fn event_scope() {
    let handler = |widget: &str, callback: &str| {
        screen(
            json!({"x": 1}),
            json!([{"type": widget, callback: {"actionType": "setState", "values": {"x": {"$state": "event.value"}}}}]),
        )
    };
    clean(handler("textField", "onChanged"));
    clean(handler("autoComplete", "onSelected"));
    only_error(
        handler("elevatedButton", "onPressed"),
        "$.children[0].onPressed.values.x",
        "`event`",
    );
}

#[test]
fn result_scope() {
    let navigate = |callback: &str| {
        screen(
            json!({"x": 1}),
            json!([{"type": "elevatedButton", "onPressed": {"actionType": "navigate", "routeName": "/r",
                callback: {"actionType": "setState", "values": {"x": {"$state": "result"}}}}}]),
        )
    };
    clean(navigate("onResult"));
    only_error(
        navigate("onCancel"),
        "$.children[0].onPressed.onCancel.values.x",
        "`result`",
    );
}

#[test]
fn item_and_index_only_inside_templates() {
    only_error(
        screen(
            json!({"x": 1}),
            json!([{"type": "text", "data": {"$state": "item.name"}}]),
        ),
        "$.children[0].data",
        "`item`",
    );
    only_error(
        screen(
            json!({"x": 1}),
            json!([{"type": "text", "data": {"$expr": "index + 1"}}]),
        ),
        "$.children[0].data",
        "`index`",
    );
    // a screen key with the same name makes it a normal read
    clean(screen(
        json!({"item": 1}),
        json!([{"type": "text", "data": {"$state": "item"}}]),
    ));
    // opaque scope: not accused
    no_error(json!({"type": "text", "data": {"$state": "item"}}));
}

#[test]
fn item_names_follow_as_and_index_as() {
    let list = |data: &str| {
        screen(
            json!({"todos": []}),
            json!([{"type": "listView", "as": "todo", "indexAs": "i", "items": {"$state": "todos"},
                "itemTemplate": {"type": "text", "data": {"$expr": data}}}]),
        )
    };
    clean(list("todo.name + i"));
    // the default names are not bound once renamed
    only_error(
        list("item.name"),
        "$.children[0].itemTemplate.data",
        "`item`",
    );
    // empty names mean default
    clean(screen(
        json!({"todos": []}),
        json!([{"type": "listView", "as": "", "items": {"$state": "todos"},
            "itemTemplate": {"type": "text", "data": {"$expr": "item.name + index"}}}]),
    ));
    // outside the template they are gone
    only_error(
        screen(
            json!({"todos": []}),
            json!([{"type": "listView", "items": {"$state": "todos"},
                "itemTemplate": {"type": "text"}, "separator": {"type": "text", "data": {"$state": "item"}}}]),
        ),
        "$.children[0].separator.data",
        "`item`",
    );
}

#[test]
fn nested_template_names() {
    let nested = |outer_as: &str, inner_as: &str| {
        screen(
            json!({"a": [], "b": []}),
            json!([{"type": "listView", "as": outer_as, "indexAs": "i", "items": {"$state": "a"},
                "itemTemplate": {"type": "listView", "as": inner_as, "indexAs": "j", "items": {"$state": "b"},
                    "itemTemplate": {"type": "text", "data": {"$expr": format!("{outer_as} + {inner_as}")}}}}]),
        )
    };
    clean(nested("row", "cell"));
    let report = check_screen(&nested("item", "item"));
    assert!(report.is_ok());
    let warns = warnings(&report);
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(warns[0].contains("shadows"), "{warns:?}");
    assert!(warns[0].starts_with("$.children[0].itemTemplate.itemTemplate"));
}

// ---------------------------------------------------------------- templates and per item state

#[test]
fn template_inherits_the_outer_keys() {
    let list = |action: Value| {
        screen(
            json!({"todos": [], "selected": []}),
            json!([{"type": "listView", "items": {"$state": "todos"},
                "itemTemplate": {"type": "checkBox", "value": {"$expr": "contains(selected, item.id)"},
                    "onChanged": action}}]),
        )
    };
    clean(list(
        json!({"actionType": "toggleItem", "key": "selected", "value": {"$state": "item.id"}}),
    ));
    clean(list(
        json!({"actionType": "toggleItem", "key": "selected", "by": "id", "value": {"$state": "item"},
        "condition": {"$expr": "length(selected) < 3 || contains(selected, item.id)"}}),
    ));
    only_error(
        list(json!({"actionType": "toggleItem", "key": "missing", "value": {"$state": "item.id"}})),
        "$.children[0].itemTemplate.onChanged.key",
        "undeclared state key `missing`",
    );
}

#[test]
fn writes_to_template_variables_are_errors() {
    for name in ["item", "index", "todo", "i"] {
        let value = screen(
            json!({"todos": []}),
            json!([{"type": "listView", "as": "todo", "indexAs": "i", "items": {"$state": "todos"},
                "itemTemplate": {"type": "elevatedButton", "onPressed": set(name)}}]),
        );
        if name == "item" || name == "index" {
            // not bound (renamed): falls under the undeclared key rule
            only_error(
                value,
                "$.children[0].itemTemplate.onPressed.values",
                "undeclared",
            );
        } else {
            only_error(
                value,
                "$.children[0].itemTemplate.onPressed.values",
                "template variable",
            );
        }
    }
    // dotted paths write to the root
    only_error(
        screen(
            json!({"todos": []}),
            json!([{"type": "listView", "items": {"$state": "todos"},
                "itemTemplate": {"type": "elevatedButton",
                    "onPressed": {"actionType": "toggleState", "key": "item.done"}}}]),
        ),
        "$.children[0].itemTemplate.onPressed.key",
        "template variable `item`",
    );
}

#[test]
fn template_with_its_own_state_is_a_warning() {
    let value = screen(
        json!({"todos": [], "selected": []}),
        json!([{"type": "listView", "items": {"$state": "todos"},
            "itemTemplate": {"type": "column", "state": {"local": 0}, "children": [
                {"type": "text", "data": {"$state": "item.name"}}]}}]),
    );
    let report = check_screen(&value);
    assert!(report.is_ok());
    let warns = warnings(&report);
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(warns[0].starts_with("$.children[0].itemTemplate"));
    assert!(warns[0].contains("own `state`"));

    // ... and writing an outer key from there is an error (no chaining)
    let value = screen(
        json!({"todos": [], "selected": []}),
        json!([{"type": "listView", "items": {"$state": "todos"},
            "itemTemplate": {"type": "elevatedButton", "state": {"local": 0},
                "onPressed": {"actionType": "toggleItem", "key": "selected", "value": {"$state": "item.id"}}}}]),
    );
    let report = check_screen(&value);
    assert_eq!(errors(&report).len(), 1);
    assert!(errors(&report)[0].contains("undeclared state key `selected`"));
}

#[test]
fn index_arguments_are_not_value_checked() {
    clean(screen(
        json!({"todos": []}),
        json!([{"type": "listView", "items": {"$state": "todos"}, "indexAs": "i",
            "itemTemplate": {"type": "elevatedButton", "onPressed": {
                "actionType": "removeFromState", "key": "todos", "index": {"$state": "i"}}}}]),
    ));
}

// ---------------------------------------------------------------- reads and undeclared keys

#[test]
fn reading_an_undeclared_key_is_only_a_warning() {
    let value = screen(
        json!({"a": 1}),
        json!([{"type": "text", "data": {"$state": "missing.deep"}},
               {"type": "text", "data": {"$expr": "a + other"}}]),
    );
    let report = check_screen(&value);
    assert!(report.is_ok());
    let warns = warnings(&report);
    assert_eq!(warns.len(), 2, "{warns:?}");
    assert!(warns[0].contains("`missing`") && warns[0].starts_with("$.children[0].data"));
    assert!(warns[1].contains("`other`") && warns[1].starts_with("$.children[1].data"));
}

#[test]
fn params_and_global_are_never_reported() {
    clean(screen(
        json!({"a": 1}),
        json!([{"type": "text", "data": {"$state": "params.id"}},
               {"type": "text", "data": {"$expr": "global.token + params.x"}}]),
    ));
}

#[test]
fn bindings_in_action_data_are_read_at_any_depth() {
    let action = json!({"actionType": "networkRequest", "url": "u",
        "body": {"a": {"b": [{"c": {"$state": "missing"}}]}}});
    let report = check_screen(&screen(
        json!({"a": 1}),
        json!([{"type": "elevatedButton", "onPressed": action}]),
    ));
    assert_eq!(warnings(&report).len(), 1);
    assert!(warnings(&report)[0].starts_with("$.children[0].onPressed.body.a.b[0].c"));
}

#[test]
fn data_with_a_type_key_is_not_a_widget() {
    // `value` is data: the object below must not be mistaken for a widget with `bind`
    clean(screen(
        json!({"a": []}),
        json!([{"type": "elevatedButton", "onPressed": {"actionType": "appendState", "key": "a",
            "value": {"type": "text", "bind": "x"}}}]),
    ));
}

#[test]
fn expression_scanner() {
    let read = |expr: &str| {
        let value = screen(
            json!({"declared": 1}),
            json!([{"type": "text", "data": {"$expr": expr}}]),
        );
        warnings(&check_screen(&value))
    };
    // only `x` is a read, `item` inside text is not (and would be an error otherwise)
    assert_eq!(read("'item' + x").len(), 1);
    assert!(read("'item' + x")[0].contains("`x`"));
    assert!(read("a.item")[0].contains("`a`"));
    assert_eq!(read("a.item").len(), 1);
    assert_eq!(read("declared.item").len(), 0);
    assert_eq!(read("item(declared)").len(), 0);
    assert_eq!(
        read("length(declared) > 1 && true || false == null").len(),
        0
    );
    assert_eq!(read(r"'it\'s item' + declared").len(), 0);
    assert_eq!(read(r#""say \"event\"" + declared"#).len(), 0);
    assert_eq!(read("declared[0] + declared . name").len(), 0);
    assert_eq!(read("1 + 22").len(), 0);
    // unterminated text does not panic
    assert_eq!(read("declared + 'oops").len(), 0);
}

// ---------------------------------------------------------------- secure keys

#[test]
fn body_from_state_rejects_secure_global_keys() {
    let value = json!({"type": "column",
        "globalState": {"token": {"initial": "MARKER-123", "secure": true}, "theme": "x"},
        "children": [{"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": "u",
            "bodyFromState": ["global.theme", "global.token", "local"]}}]});
    only_error(
        value.clone(),
        "$.children[0].onPressed.bodyFromState[1]",
        "secure global key `token`",
    );
    // the secure key can be declared anywhere in the response
    let value = json!({"type": "column", "children": [
        {"type": "text", "globalState": {"token": {"initial": "MARKER-123", "secure": true}}},
        {"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": "u",
            "bodyFromState": ["global.token"]}}]});
    only_error(value, "$.children[1].onPressed.bodyFromState[0]", "`token`");
    no_error(
        json!({"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": "u",
        "bodyFromState": ["global.token"]}}),
    );
}

#[test]
fn messages_never_contain_values() {
    let marker = "MARKER-SECRET-123";
    let value = json!({"type": "column",
    "globalState": {"token": {"initial": marker, "secure": true, "persist": false}},
    "state": {"response": marker, "k": {"initial": marker, "persist": true}},
    "children": [
        {"type": "text", "data": {"$expr": format!("'{marker}' + missing")}},
        {"type": "textField", "bind": "global.token", "initialValue": marker},
        {"type": "elevatedButton", "onPressed": {"actionType": "networkRequest", "url": marker,
            "body": {"x": {"$state": "undeclared"}},
            "bodyFromState": ["global.token"]}},
        {"type": "elevatedButton", "onPressed": {"actionType": "setState", "values": {"nope": marker}}}
    ]});
    let report = check_screen(&value);
    let all: Vec<String> = errors(&report)
        .into_iter()
        .chain(warnings(&report))
        .collect();
    assert!(all.len() >= 5, "{all:?}");
    for message in all {
        assert!(!message.contains(marker), "{message}");
    }
}

// ---------------------------------------------------------------- robustness

#[test]
fn malformed_json_never_panics() {
    for value in [
        json!(null),
        json!(1),
        json!("x"),
        json!([]),
        json!({"type": 5, "bind": 1, "state": 3}),
        json!({"type": "checkBox", "bind": null, "state": null, "globalState": 1, "onInit": 7}),
        json!({"actionType": "setState", "values": 3}),
        json!({"actionType": "setState", "values": {"a": null}, "key": 3}),
        json!({"actionType": "toggleItem", "key": 5}),
        json!({"actionType": "networkRequest", "bodyFromState": "x"}),
        json!({"actionType": "networkRequest", "bodyFromState": [1, null]}),
        json!({"type": "listView", "as": 3, "indexAs": null, "itemTemplate": 4}),
        json!({"type": "text", "data": {"$expr": 5}}),
        json!({"type": "text", "data": {"$state": ""}}),
        json!({"type": "text", "state": {"k": []}, "globalState": {"k": 1, "j": []}}),
    ] {
        let _ = check_screen(&value);
    }
}

#[test]
fn deep_json_is_reported_not_overflowed() {
    let mut value = json!({"type": "text"});
    for _ in 0..(MAX_CHECK_DEPTH + 50) {
        value = json!({"type": "column", "child": value});
    }
    let report = check_screen(&value);
    let errs = errors(&report);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("nested deeper"));
    // dropping a deep serde_json value is recursive as well: leak it to keep the test portable
    std::mem::forget(value);
}

// ---------------------------------------------------------------- typed integration

#[test]
fn typed_widgets_run_the_checker_through_validate_serialized() {
    let screen =
        Widget::from(
            column()
                .state(StateBlock::new().with("a", 1))
                .children([Widget::from(
                    elevated_button().on_pressed(set_state("zzz", 1)),
                )]),
        );
    let json = serde_json::to_value(&screen).unwrap();
    let mut report = ValidationReport::new();
    screen.validate_serialized(&json, &mut report);
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert!(report.errors[0]
        .message
        .contains("undeclared state key `zzz`"));

    let action = Action::from(set_state("a", 1));
    let json = serde_json::to_value(&action).unwrap();
    let mut report = ValidationReport::new();
    action.validate_serialized(&json, &mut report);
    assert!(report.is_ok());
}

#[test]
fn very_deep_expressions_are_rejected_by_validate() {
    use derust::sduix::stacx::v1::state::Expr;
    let mut expr = Expr::Int(1);
    for _ in 0..1000 {
        expr = Expr::Group(Box::new(expr));
    }
    let issues = expr.validate();
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("nested deeper"));
    let mut ok = Expr::Int(1);
    for _ in 0..50 {
        ok = Expr::Group(Box::new(ok));
    }
    assert!(ok.validate().is_empty());
}
