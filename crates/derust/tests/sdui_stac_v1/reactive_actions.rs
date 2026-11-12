//! State actions of the fork (`v1/state/actions.rs`) against `fixtures_reactive/actions`.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use serde_json::json;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn item_id() -> StatePath {
    path("item").field("id")
}

#[test]
fn action_set_state() {
    assert_reactive_fixture!("actions/set_state", Action::from(set_state("draft", "")));
}

#[test]
fn action_set_state_multi_nested() {
    let user = Data::object([
        ("name", Data::from(path("name"))),
        (
            "tags",
            Data::list([
                Data::from("a"),
                Data::from(Expr::length(path("tags")) + Expr::int(1)),
            ]),
        ),
    ]);
    assert_reactive_fixture!(
        "actions/set_state_multi_nested",
        Action::from(
            set_state("draft", "")
                .and("global.theme", "dark")
                .and("user", user)
        )
    );
}

#[test]
fn action_set_state_condition_expr() {
    assert_reactive_fixture!(
        "actions/set_state_condition_expr",
        Action::from(set_state("sent", true).condition(Expr::length(path("draft")).gt(0)))
    );
}

#[test]
fn action_increment_state() {
    assert_reactive_fixture!(
        "actions/increment_state",
        Action::from(increment_state("total"))
    );
}

#[test]
fn action_increment_state_by_min_max() {
    assert_reactive_fixture!(
        "actions/increment_state_by_min_max",
        Action::from(increment_state("total").by(2).min(0).max(6))
    );
}

#[test]
fn action_increment_state_bound() {
    assert_reactive_fixture!(
        "actions/increment_state_bound",
        Action::from(
            increment_state("global.total")
                .by(path("step"))
                .min(Expr::from(path("floor")) + Expr::int(1))
                .max(path("limit"))
                .condition(path("enabled"))
        )
    );
}

#[test]
fn action_decrement_state() {
    assert_reactive_fixture!(
        "actions/decrement_state",
        Action::from(decrement_state("total"))
    );
}

#[test]
fn action_decrement_state_by_min_max() {
    assert_reactive_fixture!(
        "actions/decrement_state_by_min_max",
        Action::from(decrement_state("total").by(0.5).min(0).max(10))
    );
}

#[test]
fn action_toggle_state() {
    assert_reactive_fixture!("actions/toggle_state", Action::from(toggle_state("flag")));
}

#[test]
fn action_toggle_state_global() {
    assert_reactive_fixture!(
        "actions/toggle_state_global",
        Action::from(toggle_state("global.flag").condition(!Expr::from(path("busy"))))
    );
}

#[test]
fn action_reset_state() {
    assert_reactive_fixture!("actions/reset_state", Action::from(reset_state()));
}

#[test]
fn action_reset_state_keys() {
    assert_reactive_fixture!(
        "actions/reset_state_keys",
        Action::from(reset_state().keys(["flag", "global.theme"]))
    );
}

#[test]
fn action_append_state() {
    assert_reactive_fixture!("actions/append_state", Action::from(append_state("tags")));
}

#[test]
fn action_append_state_value() {
    assert_reactive_fixture!(
        "actions/append_state_value",
        Action::from(append_state("tags").value(path("draft")))
    );
}

#[test]
fn action_append_state_object() {
    let todo = Data::object([
        ("id", Data::from(Expr::length(path("todos")) + Expr::int(1))),
        ("title", Data::from(path("draft"))),
        ("done", Data::from(false)),
    ]);
    assert_reactive_fixture!(
        "actions/append_state_object",
        Action::from(append_state("todos").value(todo))
    );
}

#[test]
fn action_remove_from_state_index() {
    assert_reactive_fixture!(
        "actions/remove_from_state_index",
        Action::from(remove_from_state_at("tags", 0))
    );
}

#[test]
fn action_remove_from_state_index_binding() {
    // `index` is a template variable (read only, not a state key)
    assert_reactive_fixture!(
        "actions/remove_from_state_index_binding",
        Action::from(remove_from_state_at("todos", path("index")))
    );
}

#[test]
fn action_remove_from_state_value() {
    assert_reactive_fixture!(
        "actions/remove_from_state_value",
        Action::from(
            remove_from_state_value("tags", "none")
                .condition(Expr::contains(path("tags"), Expr::text("none")))
        )
    );
}

#[test]
fn action_remove_from_state_index_and_value() {
    assert_reactive_fixture!(
        "actions/remove_from_state_index_and_value",
        Action::from(remove_from_state_at("tags", 1).also_value("x"))
    );
}

#[test]
fn action_toggle_item() {
    assert_reactive_fixture!(
        "actions/toggle_item",
        Action::from(toggle_item("tags").value("red"))
    );
}

#[test]
fn action_toggle_item_item_id() {
    assert_reactive_fixture!(
        "actions/toggle_item_item_id",
        Action::from(toggle_item("selected").value(item_id()))
    );
}

#[test]
fn action_toggle_item_by() {
    assert_reactive_fixture!(
        "actions/toggle_item_by",
        Action::from(toggle_item("selected").value(path("item")).by("id"))
    );
}

#[test]
fn action_toggle_item_limit() {
    // aggregate over a plain screen key, readable from inside a list template
    let limit = Expr::length(path("selected"))
        .lt(3)
        .or(Expr::contains(path("selected"), item_id()));
    assert_reactive_fixture!(
        "actions/toggle_item_limit",
        Action::from(toggle_item("selected").value(item_id()).condition(limit))
    );
}

#[test]
fn action_cancel_request() {
    assert_reactive_fixture!(
        "actions/cancel_request",
        Action::from(cancel_request("users"))
    );
}

#[test]
fn action_none() {
    assert_reactive_fixture!("actions/none", Action::from(none_action()));
}

#[test]
fn action_none_condition() {
    assert_reactive_fixture!(
        "actions/none_condition",
        Action::from(none_action().condition(path("ready")))
    );
}

#[test]
fn action_multi_action_contract() {
    let actions: Vec<Action> = vec![
        append_state("tags").value(path("draft")).into(),
        increment_state("total").by(2).max(6).into(),
        toggle_state("flag").into(),
        toggle_item("tags").value("red").into(),
        remove_from_state_value("tags", "none")
            .condition(Expr::contains(path("tags"), Expr::text("none")))
            .into(),
        reset_state().keys(["flag"]).into(),
        set_state("draft", "").into(),
    ];
    assert_reactive_fixture!(
        "actions/multi_action_contract",
        Action::from(MultiAction::new().sync(true).actions(actions))
    );
}

#[test]
fn action_on_init() {
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("count", 0))
            .on_init(vec![
                Action::from(set_state("count", 1)),
                Action::from(increment_state("count")),
            ]),
    );
    assert_reactive_fixture!("actions/on_init", screen);
    assert!(screen.validate().is_empty());
}

#[test]
fn action_type_names() {
    let names: Vec<&str> = [
        Action::from(set_state("a", 1)),
        Action::from(increment_state("a")),
        Action::from(decrement_state("a")),
        Action::from(toggle_state("a")),
        Action::from(reset_state()),
        Action::from(append_state("a")),
        Action::from(remove_from_state_at("a", 0)),
        Action::from(toggle_item("a")),
        Action::from(cancel_request("a")),
        Action::from(none_action()),
    ]
    .iter()
    .map(Action::type_name)
    .collect();
    assert_eq!(
        names,
        [
            "setState",
            "incrementState",
            "decrementState",
            "toggleState",
            "resetState",
            "appendState",
            "removeFromState",
            "toggleItem",
            "cancelRequest",
            "none"
        ]
    );
}

#[test]
fn action_absent_optionals_are_omitted() {
    let value = serde_json::to_value(Action::from(increment_state("n"))).unwrap();
    assert_eq!(value, json!({"actionType": "incrementState", "key": "n"}));
    let value = serde_json::to_value(Action::from(toggle_item("n"))).unwrap();
    assert_eq!(value, json!({"actionType": "toggleItem", "key": "n"}));
}

#[test]
fn action_validation() {
    assert!(Action::from(set_state("a", 1)).validate().is_empty());
    assert!(!Action::from(set_state("", 1)).validate().is_empty());
    assert!(!Action::from(toggle_state("")).validate().is_empty());
    assert!(!Action::from(toggle_state("global.")).validate().is_empty());
    assert!(!Action::from(cancel_request(" ")).validate().is_empty());
    assert!(!Action::from(toggle_item("a").by("")).validate().is_empty());
    // `Data` child: a one-key object that looks like a binding
    let fake = Data::object([("$state", "x")]);
    assert!(!Action::from(append_state("a").value(fake))
        .validate()
        .is_empty());
    // `Reactive` child: an expression that cannot be printed
    let bad = Expr::int(1) / Expr::num(f64::NAN);
    assert!(!Action::from(increment_state("a").by(bad))
        .validate()
        .is_empty());
    assert!(
        !Action::from(none_action().condition(Expr::num(f64::INFINITY)))
            .validate()
            .is_empty()
    );
}
