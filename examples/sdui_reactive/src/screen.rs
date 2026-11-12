//! Reactive SDUI screens, built only with typed values of `derust::sduix::stacx::v1::prelude`
//! (no JSON strings, no expression strings): state blocks, `bind`, typed expressions, state
//! actions, `networkRequest` callbacks, `navigate` with params/result and list templates.
//!
//! The reactive vocabulary needs the derust-stac fork on the client (branch
//! `estado_compartilhado_reativo`); the official Stac ignores or rejects it.
//!
//! Also compiled by the `derust` test-suite (`tests/sdui_stac_v1/example_reactive_test.rs`), which
//! compares every screen with `fixtures_reactive/screens/example_*.json` (parsed by the fork in CI).

use derust::sduix::stacx::v1::prelude::*;
use std::collections::BTreeMap;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn label(data: impl Into<Dyn<String>>) -> Widget {
    Widget::from(text(data))
}

/// `length(trim(name)) < 3`
fn name_too_short() -> Expr {
    Expr::length(Expr::trim(path("name"))).lt(3)
}

/// Form: `bind` on a text field and a check box, a hint shown by an expression, a button enabled
/// by an expression and a conditional `setState`.
pub fn form_screen() -> Widget {
    Widget::from(
        scaffold().app_bar(app_bar().title(text("Form"))).body(
            column()
                .state(
                    StateBlock::new()
                        .with("name", "")
                        .with("accepted", false)
                        .with("saved", false),
                )
                .children([
                    Widget::from(text_field().bind("name")),
                    Widget::from(check_box().bind("accepted")),
                    Widget::from(text("Name is too short").visible(name_too_short())),
                    Widget::from(
                        elevated_button()
                            .enabled(Expr::from(path("accepted")).and(!name_too_short()))
                            .child(label("Save"))
                            .on_pressed(Action::from(
                                set_state("saved", true).condition(!Expr::from(path("saved"))),
                            )),
                    ),
                    Widget::from(text("Saved").visible(path("saved"))),
                ]),
        ),
    )
}

/// List: `appendState` to add, `removeFromState` by `index` to remove, `ListView::template`.
pub fn list_screen() -> Widget {
    let add = MultiAction::new().sync(true).actions(vec![
        Action::from(
            append_state("todos")
                .value(Data::object([
                    ("id", Data::from(path("nextId"))),
                    ("name", Data::from(path("draft"))),
                ]))
                .condition(Expr::length(Expr::trim(path("draft"))).gt(0)),
        ),
        Action::from(increment_state("nextId")),
        Action::from(set_state("draft", "")),
    ]);
    Widget::from(
        scaffold().app_bar(app_bar().title(text("Todos"))).body(
            column()
                .state(
                    StateBlock::new()
                        .with(
                            "todos",
                            Data::list([Data::object([
                                ("id", Data::Int(1)),
                                ("name", Data::from("Buy milk")),
                            ])]),
                        )
                        .with("draft", "")
                        .with("nextId", 2),
                )
                .children([
                    Widget::from(text_field().bind("draft")),
                    Widget::from(
                        elevated_button()
                            .child(label("Add"))
                            .on_pressed(Action::from(add)),
                    ),
                    Widget::from(text("Nothing to do").visible(Expr::length(path("todos")).eq_(0))),
                    Widget::from(
                        list_view().shrink_wrap(true).template(
                            ItemTemplate::new(path("todos"))
                                .item_as("todo")
                                .index_as("i"),
                            |vars| {
                                let title = (Expr::from(vars.index()) + 1).grouped()
                                    + ". "
                                    + vars.item_field("name");
                                Widget::from(list_tile().title(label(title)).trailing(
                                    Widget::from(text_button().child(label("Done")).on_pressed(
                                        Action::from(remove_from_state_at("todos", vars.index())),
                                    )),
                                ))
                            },
                        ),
                    ),
                ]),
        ),
    )
}

/// `networkRequest` with the four callbacks, a request key and `cancelRequest`.
pub fn network_screen() -> Widget {
    let request = NetworkRequest::new("https://api.example.com/users")
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
    Widget::from(
        scaffold().app_bar(app_bar().title(text("Users"))).body(
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
                            .enabled(path("loading"))
                            .child(label("Cancel"))
                            .on_pressed(Action::from(cancel_request("users"))),
                    ),
                    Widget::from(text("Could not load").visible(path("failed"))),
                    label(Expr::length(path("users")) + " users"),
                ]),
        ),
    )
}

/// `navigate` with `params`; the destination pops with a `result`; the origin handles
/// `onResult` and `onCancel`.
pub fn navigation_screen() -> Widget {
    let confirm = NavigateAction::new()
        .navigation_style(NavigationStyle::Pop)
        .result(Data::object([
            ("name", Data::from(StatePath::params().field("name"))),
            ("qty", Data::from(path("qty"))),
        ]));
    let destination = column().state(StateBlock::new().with("qty", 1)).children([
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
        .widget_json(Widget::from(destination))
        .on_result(Action::from(
            append_state("picked").value(StatePath::result()),
        ))
        .on_cancel(Action::from(set_state("cancelled", true)));
    Widget::from(
        column()
            .state(
                StateBlock::new()
                    .with("product", "Coffee")
                    .with("picked", Data::List(vec![]))
                    .with("cancelled", false),
            )
            .children([
                Widget::from(
                    elevated_button()
                        .child(label("Choose"))
                        .on_pressed(Action::from(navigate)),
                ),
                label(Expr::length(path("picked")) + " picked"),
                Widget::from(text("Cancelled").visible(path("cancelled"))),
            ]),
    )
}

/// A screen that breaks a contextual rule: the button writes a key that is not declared in the
/// `state` block. `SduiResponse::try_new` rejects it with HTTP 500 and a message that locates the
/// problem (`$.children[0].onPressed...`).
pub fn invalid_screen() -> Widget {
    Widget::from(
        column()
            .state(StateBlock::new().with("count", 0))
            .children([Widget::from(
                elevated_button()
                    .child(label("Oops"))
                    .on_pressed(Action::from(set_state("missing", 1))),
            )]),
    )
}
