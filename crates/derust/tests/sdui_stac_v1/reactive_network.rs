//! `networkRequest` and `navigate` extended by the fork against `fixtures_reactive/actions`.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use serde_json::json;
use std::collections::BTreeMap;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn text_of(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn network_request_contract_example() {
    let request = NetworkRequest::new("https://example.com/api/users")
        .method(Method::Get)
        .request_key("users")
        .on_start(Action::from(
            set_state("loading", true).and("failed", false),
        ))
        .on_success(Action::from(set_state(
            "users",
            StatePath::response().field("body").field("items"),
        )))
        .on_error(Action::from(set_state("failed", true)))
        .on_finally(Action::from(set_state("loading", false)));
    assert_reactive_fixture!(
        "actions/network_request_contract",
        Action::from(request.clone())
    );
    assert!(request.validate().is_empty());
}

#[test]
fn network_request_callback_lists_keep_order() {
    let request = NetworkRequest::new("https://example.com/api/save")
        .method(Method::Post)
        .on_start(vec![
            Action::from(set_state("step", 1)),
            Action::from(toggle_state("busy")),
            Action::from(set_state("step", 3)),
        ])
        .on_success(vec![
            Action::from(increment_state("saved")),
            Action::from(none_action()),
        ]);
    assert_reactive_fixture!(
        "actions/network_request_callbacks_list",
        Action::from(request)
    );
}

#[test]
fn network_request_body_with_deep_bindings() {
    let body = Data::object([
        (
            "user",
            Data::object([
                ("name", Data::from(path("user").field("name"))),
                (
                    "tags",
                    Data::list([Data::from("a"), Data::from(path("tags").index(0))]),
                ),
            ]),
        ),
        ("total", Data::from(Expr::from(path("price")) * path("qty"))),
        ("note", Data::from("plain")),
    ]);
    let request = NetworkRequest::new("https://example.com/api/orders")
        .method(Method::Post)
        .headers(BTreeMap::from([(
            "X-Trace".to_string(),
            Data::from(path("trace")),
        )]))
        .query_parameters(BTreeMap::from([(
            "page".to_string(),
            Data::from(Expr::from(path("page")) + 1),
        )]))
        .body(body);
    assert_reactive_fixture!(
        "actions/network_request_body_deep",
        Action::from(request.clone())
    );
    assert!(request.validate().is_empty());
}

#[test]
fn network_request_literal_body_shaped_like_a_binding_is_an_error() {
    let body = Data::object([("$state", Data::from("a.b"))]);
    let issues = NetworkRequest::new("u").body(body).validate();
    assert_eq!(issues.len(), 1, "{}", text_of(&issues));
    assert_eq!(issues[0].path, "$.body");

    let nested = Data::object([("k", Data::object([("$expr", Data::from("1 + 1"))]))]);
    let issues = NetworkRequest::new("u").body(nested).validate();
    assert_eq!(issues.len(), 1, "{}", text_of(&issues));
    assert_eq!(issues[0].path, "$.body.k");

    let headers = BTreeMap::from([("h".to_string(), Data::object([("$state", Data::from("x"))]))]);
    assert_eq!(
        NetworkRequest::new("u").headers(headers).validate().len(),
        1
    );
}

#[test]
fn network_request_body_from_state_with_global_key() {
    let request = NetworkRequest::new("https://example.com/api/login")
        .method(Method::Post)
        .body_from_state(vec![BindKey::screen("email"), BindKey::global("token")]);
    assert_reactive_fixture!(
        "actions/network_request_body_from_state",
        Action::from(request.clone())
    );
    assert!(request.validate().is_empty());
    let empty_key = NetworkRequest::new("u").body_from_state(vec![BindKey::screen("")]);
    assert_eq!(empty_key.validate().len(), 1);
}

#[test]
fn network_request_callbacks_are_validated() {
    let bad = Action::from(set_state("a", Data::object([("$state", Data::from("x"))])));
    let issues = NetworkRequest::new("u").on_error(bad).validate();
    assert_eq!(issues.len(), 1, "{}", text_of(&issues));
    assert!(
        issues[0].path.starts_with("$.onError"),
        "{}",
        issues[0].path
    );
}

#[test]
fn navigate_contract_example() {
    let confirm = NavigateAction::new()
        .navigation_style(NavigationStyle::Pop)
        .result(Data::object([
            ("name", Data::from(StatePath::params().field("name"))),
            ("qty", Data::from(path("qty"))),
        ]));
    let screen = column().state(StateBlock::new().with("qty", 1)).children([
        Widget::from(text(Expr::text("Buy ") + StatePath::params().field("name"))),
        Widget::from(
            elevated_button()
                .child(Widget::from(text("Confirm")))
                .on_pressed(Action::from(confirm.clone())),
        ),
    ]);
    let navigate = NavigateAction::new()
        .navigation_style(NavigationStyle::Push)
        .params(BTreeMap::from([(
            "name".to_string(),
            Data::from(path("product")),
        )]))
        .widget_json(Widget::from(screen))
        .on_result(Action::from(
            append_state("picked").value(StatePath::result()),
        ))
        .on_cancel(Action::from(none_action()));
    assert_reactive_fixture!("actions/navigate_contract", Action::from(navigate.clone()));
    // the pop inside is fine; the contract example itself sets both callbacks, which `Validate`
    // rejects (the contract text says they are mutually exclusive)
    assert!(confirm.validate().is_empty());
    assert_eq!(navigate.validate().len(), 1);
}

#[test]
fn navigate_result_absent_vs_null() {
    let absent = NavigateAction::new().navigation_style(NavigationStyle::Pop);
    assert_reactive_fixture!("actions/navigate_pop_result_absent", Action::from(absent));
    let null = NavigateAction::new()
        .navigation_style(NavigationStyle::Pop)
        .result(Data::Null);
    assert_reactive_fixture!(
        "actions/navigate_pop_result_null",
        Action::from(null.clone())
    );
    assert!(null.validate().is_empty());
}

#[test]
fn navigate_on_result_alone_and_on_cancel_alone() {
    let on_result = NavigateAction::new()
        .navigation_style(NavigationStyle::Push)
        .route_name("/pick")
        .arguments(Data::object([("id", Data::from(path("id")))]))
        .on_result(Action::from(set_state("picked", StatePath::result())));
    assert_reactive_fixture!(
        "actions/navigate_on_result",
        Action::from(on_result.clone())
    );
    assert!(on_result.validate().is_empty());

    let on_cancel = NavigateAction::new()
        .route_name("/pick")
        .params(BTreeMap::from([("a".to_string(), Data::from(1))]))
        .on_cancel(Action::from(none_action()));
    assert_reactive_fixture!(
        "actions/navigate_on_cancel",
        Action::from(on_cancel.clone())
    );
    assert!(on_cancel.validate().is_empty());
}

#[test]
fn navigate_on_result_with_on_cancel_is_an_error() {
    let both = NavigateAction::new()
        .on_result(Action::from(none_action()))
        .on_cancel(Action::from(none_action()));
    let issues = both.validate();
    assert_eq!(issues.len(), 1, "{}", text_of(&issues));
    assert!(issues[0].message.contains("mutually exclusive"));
}

#[test]
fn navigate_result_depends_on_the_style() {
    let with = |style: Option<NavigationStyle>| {
        let mut nav = NavigateAction::new().result(Data::Null);
        if let Some(style) = style {
            nav = nav.navigation_style(style);
        }
        nav.validate()
    };
    // no style counts as push
    assert_eq!(with(None).len(), 1);
    assert_eq!(with(None)[0].path, "$.result");
    for bad in [
        NavigationStyle::Push,
        NavigationStyle::PushAndRemoveAll,
        NavigationStyle::PopAll,
        NavigationStyle::PushNamed,
        NavigationStyle::PushNamedAndRemoveAll,
    ] {
        assert_eq!(with(Some(bad)).len(), 1, "{bad:?}");
    }
    for ok in [
        NavigationStyle::Pop,
        NavigationStyle::PushReplacement,
        NavigationStyle::PushReplacementNamed,
    ] {
        assert!(with(Some(ok)).is_empty(), "{ok:?}");
    }
    // a bound style is only known at runtime
    let bound = NavigateAction::new()
        .result(Data::Null)
        .navigation_style(path("style"));
    assert!(bound.validate().is_empty());
}

#[test]
fn navigate_params_and_result_are_checked_for_binding_shapes() {
    let nav = NavigateAction::new()
        .navigation_style(NavigationStyle::Pop)
        .result(Data::object([("$state", Data::from("a"))]));
    assert_eq!(nav.validate().len(), 1);
    let nav = NavigateAction::new().params(BTreeMap::from([(
        "p".to_string(),
        Data::object([("$expr", Data::from("1"))]),
    )]));
    assert_eq!(nav.validate().len(), 1);
}

#[test]
fn actions_serializes_object_or_array() {
    let one = Actions::from(Action::from(none_action()));
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        json!({"actionType": "none"})
    );
    let many = Actions::from(vec![
        Action::from(none_action()),
        Action::from(toggle_state("a")),
    ]);
    assert_eq!(
        serde_json::to_value(&many).unwrap(),
        json!([{"actionType": "none"}, {"actionType": "toggleState", "key": "a"}])
    );
}
