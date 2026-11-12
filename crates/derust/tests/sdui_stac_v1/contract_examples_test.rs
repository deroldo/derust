//! The examples of the contract documentation of the fork (`state_reference.mdx` and
//! `bindings_and_expressions.mdx`), reproduced with the typed API. The JSON under
//! `fixtures_reactive/screens/contract_*.json` is copied literally from the documentation; the
//! serialized screen must be equal to it (key order is irrelevant) and must pass both validation
//! layers without errors or warnings.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use derust::sduix::stacx::v1::state::check_screen;
use std::collections::BTreeMap;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn label(data: impl Into<Dyn<String>>) -> Widget {
    Widget::from(text(data))
}

/// Both validation layers pass: the typed one and the check over the serialized JSON.
fn assert_valid(screen: &Widget) {
    let issues = screen.validate();
    assert!(issues.is_empty(), "typed validation: {issues:?}");
    assert_verifier_clean(screen);
}

/// Only the check over the serialized JSON.
fn assert_verifier_clean(screen: &Widget) {
    let json = serde_json::to_value(screen).expect("serializes");
    let report = check_screen(&json);
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(
        report.warnings.is_empty(),
        "warnings: {:?}",
        report.warnings
    );
}

fn big_name_ok() -> Expr {
    Expr::length(Expr::trim(path("name"))).ge(3)
}

#[test]
fn bindings_state_and_expr_example() {
    let screen = Widget::from(
        column()
            .state(
                StateBlock::new()
                    .with("price", 4)
                    .with("qty", 3)
                    .with("user", Data::object([("name", Data::from("Ana"))])),
            )
            .children([
                label(Binding::State(path("user").field("name"))),
                label(Expr::text("Total: ") + Expr::from(path("price")) * path("qty")),
            ]),
    );
    assert_reactive_fixture!("screens/contract_bindings_user_total", screen);
    assert_valid(&screen);
}

#[test]
fn expression_total_example() {
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("total", 3))
            .children([label(Expr::text("Total: ") + path("total"))]),
    );
    assert_reactive_fixture!("screens/contract_expr_total", screen);
    assert_valid(&screen);
}

#[test]
fn expression_trim_ternary_example() {
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("name", "  Ana "))
            .children([label(Expr::cond(
                big_name_ok(),
                Expr::text("ok"),
                Expr::text("too short"),
            ))]),
    );
    assert_reactive_fixture!("screens/contract_expr_trim_ternary", screen);
    assert_valid(&screen);
}

#[test]
fn expression_condition_example() {
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("name", "").with("saved", false))
            .children([Widget::from(
                elevated_button()
                    .child(label("Save"))
                    .on_pressed(Action::from(
                        set_state("saved", true).condition(big_name_ok()),
                    )),
            )]),
    );
    assert_reactive_fixture!("screens/contract_expr_condition", screen);
    assert_valid(&screen);
}

#[test]
fn contract_global_state_and_on_init() {
    let screen = Widget::from(
        column()
            .global_state(
                GlobalStateBlock::new()
                    .with("userName", GlobalEntry::value("Ana"))
                    .with("theme", GlobalEntry::declared("light").persist(true))
                    .with("token", GlobalEntry::declared("").secure(true)),
            )
            .state(
                StateBlock::new()
                    .with("total", 0)
                    .with("ready", false)
                    .with(
                        "greeting",
                        Expr::text("Hi ") + StatePath::global("userName"),
                    ),
            )
            .on_init(vec![
                Action::from(set_state("ready", true)),
                Action::from(increment_state("total").by(10)),
            ])
            .children([
                label(Binding::State(path("greeting"))),
                Widget::from(text(Expr::text("Total: ") + path("total")).visible(path("ready"))),
            ]),
    );
    assert_reactive_fixture!("screens/contract_global_state", screen);
    assert_valid(&screen);
}

#[test]
fn contract_form_with_bind() {
    let screen = Widget::from(
        form()
            .state(
                StateBlock::new()
                    .with("name", "")
                    .with("accepted", false)
                    .with("saved", false),
            )
            .child(
                column().children([
                    Widget::from(text_field().bind(BindKey::screen("name"))),
                    Widget::from(check_box().bind(BindKey::screen("accepted"))),
                    Widget::from(
                        text("Name is too short")
                            .visible(Expr::length(Expr::trim(path("name"))).lt(3)),
                    ),
                    Widget::from(
                        elevated_button()
                            .enabled(Expr::from(path("accepted")).and(big_name_ok()))
                            .child(label("Save"))
                            .on_pressed(Action::from(
                                set_state("saved", true).condition(!Expr::from(path("saved"))),
                            )),
                    ),
                    Widget::from(text("Saved").visible(path("saved"))),
                ]),
            ),
    );
    assert_reactive_fixture!("screens/contract_form_bind", screen);
    assert_valid(&screen);
}

#[test]
fn contract_multi_action() {
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
    let screen = Widget::from(
        column()
            .state(
                StateBlock::new()
                    .with("tags", Data::list(["red"]))
                    .with("draft", "blue")
                    .with("total", 5)
                    .with("flag", false),
            )
            .children([
                label(Expr::length(path("tags")) + " tags, total " + path("total")),
                Widget::from(
                    elevated_button()
                        .child(label("Run"))
                        .on_pressed(Action::from(MultiAction::new().sync(true).actions(actions))),
                ),
            ]),
    );
    assert_reactive_fixture!("screens/contract_multi_action", screen);
    assert_valid(&screen);
}

#[test]
fn contract_network_request() {
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
    let screen = Widget::from(
        column()
            .state(
                StateBlock::new()
                    .with("users", Data::List(vec![]))
                    .with("loading", false)
                    .with("failed", false),
            )
            .children([
                Widget::from(
                    elevated_button()
                        .enabled(!Expr::from(path("loading")))
                        .child(label("Load"))
                        .on_pressed(Action::from(request)),
                ),
                Widget::from(
                    elevated_button()
                        .child(label("Cancel"))
                        .on_pressed(Action::from(cancel_request("users"))),
                ),
                Widget::from(text("Could not load").visible(path("failed"))),
                label(Expr::length(path("users")) + " users"),
            ]),
    );
    assert_reactive_fixture!("screens/contract_network_request", screen);
    assert_valid(&screen);
}

#[test]
fn contract_navigate_with_params_and_result() {
    let confirm = NavigateAction::new()
        .navigation_style(NavigationStyle::Pop)
        .result(Data::object([
            ("name", Data::from(StatePath::params().field("name"))),
            ("qty", Data::from(path("qty"))),
        ]));
    let target = column().state(StateBlock::new().with("qty", 1)).children([
        label(Expr::text("Buy ") + StatePath::params().field("name")),
        Widget::from(
            elevated_button()
                .child(label("Confirm"))
                .on_pressed(Action::from(confirm)),
        ),
    ]);
    let navigate = NavigateAction::new()
        .navigation_style(NavigationStyle::Push)
        .params(BTreeMap::from([(
            "name".to_string(),
            Data::from(path("product")),
        )]))
        .widget_json(Widget::from(target))
        .on_result(Action::from(
            append_state("picked").value(StatePath::result()),
        ))
        .on_cancel(Action::from(none_action()));
    let screen = Widget::from(
        column()
            .state(
                StateBlock::new()
                    .with("product", "Coffee")
                    .with("picked", Data::List(vec![])),
            )
            .children([
                Widget::from(
                    elevated_button()
                        .child(label("Choose"))
                        .on_pressed(Action::from(navigate)),
                ),
                label(Expr::length(path("picked")) + " picked"),
            ]),
    );
    assert_reactive_fixture!("screens/contract_navigate", screen);
    // `onResult` and `onCancel` coexist (the contract text calling them mutually exclusive
    // needs a correction in the fork); the literal example validates cleanly.
    let issues = screen.validate();
    assert!(issues.is_empty(), "{issues:?}");
    assert_verifier_clean(&screen);
}

#[test]
fn contract_list_with_template() {
    let todos = Data::list([
        Data::object([("id", Data::Int(1)), ("name", Data::from("Buy milk"))]),
        Data::object([("id", Data::Int(2)), ("name", Data::from("Walk dog"))]),
    ]);
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("todos", todos))
            .children([
                Widget::from(
                    text("Nothing to do").visible(Expr::length(path("todos")).eq_(Expr::int(0))),
                ),
                Widget::from(
                    list_view().shrink_wrap(true).template(
                        ItemTemplate::new(path("todos"))
                            .item_as("todo")
                            .index_as("i"),
                        |vars| {
                            let title = (Expr::from(vars.index()) + 1).grouped()
                                + ". "
                                + vars.item_field("name");
                            Widget::from(list_tile().title(label(title)).trailing(Widget::from(
                                text_button().child(label("Done")).on_pressed(Action::from(
                                    remove_from_state_at("todos", vars.index()),
                                )),
                            )))
                        },
                    ),
                ),
                Widget::from(
                    elevated_button()
                        .child(label("Add"))
                        .on_pressed(Action::from(append_state("todos").value(Data::object([
                            ("id", Data::Int(3)),
                            ("name", Data::from("New task")),
                        ])))),
                ),
            ]),
    );
    assert_reactive_fixture!("screens/contract_list", screen);
    assert_valid(&screen);
}

/// The per-item state pattern (decision D7): selection kept as a set of ids in the screen state,
/// at most three selected. Same fixture as `reactive_lists.rs`, here also through the verifier.
#[test]
fn selection_limit_example_passes_the_verifier() {
    let json = crate::load_reactive_fixture("lists/selection_limit");
    let report = check_screen(&json);
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(
        report.warnings.is_empty(),
        "warnings: {:?}",
        report.warnings
    );
}
