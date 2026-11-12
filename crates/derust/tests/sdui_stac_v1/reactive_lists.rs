//! List templates of the fork (`items`, `itemTemplate`, `as`, `indexAs`) against
//! `fixtures_reactive/lists`.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use derust::sduix::stacx::v1::state::ItemTemplate;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn label(data: impl Into<Dyn<String>>) -> Widget {
    Widget::from(text(data))
}

fn bound_text(path: StatePath) -> Widget {
    label(Binding::State(path))
}

fn expr_text(expr: Expr) -> Widget {
    label(Binding::Expr(expr))
}

fn messages(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn contract_list_example() {
    let screen = column()
        .state(StateBlock::new().with(
            "todos",
            Data::list([
                Data::object([("id", Data::Int(1)), ("name", Data::from("Buy milk"))]),
                Data::object([("id", Data::Int(2)), ("name", Data::from("Walk dog"))]),
            ]),
        ))
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
                        Widget::from(list_tile().title(expr_text(title)).trailing(Widget::from(
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
        ]);
    assert_reactive_fixture!("lists/contract", Widget::from(screen.clone()));
    assert!(
        screen.validate().is_empty(),
        "{}",
        messages(&screen.validate())
    );
}

#[test]
fn default_names_are_item_and_index() {
    let list = list_view().template(ItemTemplate::new(path("todos")), |vars| {
        assert_eq!(vars.item().to_string(), "item");
        assert_eq!(vars.index().to_string(), "index");
        let title = (Expr::from(vars.index()) + 1).grouped() + ". " + vars.item_field("name");
        expr_text(title)
    });
    assert_reactive_fixture!("lists/default_names", Widget::from(list));
}

#[test]
fn empty_names_use_defaults_and_are_not_written() {
    let template = ItemTemplate::new(path("todos")).item_as("").index_as("");
    let list = list_view().template(template, |vars| {
        assert_eq!(vars.item().to_string(), "item");
        assert_eq!(vars.index().to_string(), "index");
        bound_text(vars.item_field("name"))
    });
    assert_reactive_fixture!("lists/empty_names", Widget::from(list.clone()));
    assert!(list.validate().is_empty());

    // the plain setters accept an empty text too, and do not write the key either
    let direct = list_view().item_name("").index_name("");
    let json = serde_json::to_value(Widget::from(direct.clone())).expect("serializes");
    assert!(json.get("as").is_none() && json.get("indexAs").is_none());
    assert!(direct.validate().is_empty());
}

#[test]
fn renaming_keeps_every_reference_coherent() {
    let list = list_view().template(
        ItemTemplate::new(path("rows")).item_as("row").index_as("n"),
        |vars| {
            assert_eq!(vars.item().to_string(), "row");
            assert_eq!(vars.item_field("name").to_string(), "row.name");
            Widget::from(
                list_tile()
                    .title(bound_text(vars.item_field("name")))
                    .subtitle(expr_text(Expr::from(vars.index()) + 1))
                    .trailing(Widget::from(text_button().child(label("x")).on_pressed(
                        Action::from(remove_from_state_at("rows", vars.index())),
                    ))),
            )
        },
    );
    assert_reactive_fixture!("lists/renamed", Widget::from(list));
}

#[test]
fn nested_lists_with_distinct_names_see_both_levels() {
    let list = list_view().template(
        ItemTemplate::new(path("groups"))
            .item_as("group")
            .index_as("g"),
        |outer| {
            let entries = outer.item_field("entries");
            let group = outer.clone();
            let inner = list_view().shrink_wrap(true).template(
                ItemTemplate::new(entries).item_as("entry").index_as("e"),
                move |vars| {
                    let line = Expr::from(group.item_field("title"))
                        + " / "
                        + vars.item_field("name")
                        + " #"
                        + (Expr::from(group.index()) + 1).grouped()
                        + "."
                        + (Expr::from(vars.index()) + 1).grouped();
                    expr_text(line)
                },
            );
            Widget::from(
                column().children([bound_text(outer.item_field("title")), Widget::from(inner)]),
            )
        },
    );
    assert_reactive_fixture!("lists/nested", Widget::from(list.clone()));
    assert!(list.validate().is_empty(), "{}", messages(&list.validate()));
}

#[test]
fn item_template_coexists_with_children() {
    let list = list_view()
        .children([label("fallback")])
        .template(ItemTemplate::new(path("todos")), |vars| {
            bound_text(vars.item_field("name"))
        });
    assert_reactive_fixture!("lists/with_children", Widget::from(list));
}

#[test]
fn items_literal_binding_and_expression() {
    let literal = list_view().template(
        ItemTemplate::new(vec![
            Data::object([("name", Data::from("a"))]),
            Data::object([("name", Data::from("b"))]),
        ]),
        |vars| bound_text(vars.item_field("name")),
    );
    assert_reactive_fixture!("lists/items_literal", Widget::from(literal.clone()));
    assert!(literal.validate().is_empty());

    let expression = list_view()
        .template(ItemTemplate::new(Expr::from(path("filtered"))), |vars| {
            bound_text(vars.item_field("name"))
        });
    assert_reactive_fixture!("lists/items_expression", Widget::from(expression));

    // the plain setter takes the same inputs
    let json = serde_json::to_value(Widget::from(list_view().items(path("todos")))).expect("json");
    assert_eq!(json["items"], serde_json::json!({"$state": "todos"}));
}

#[test]
fn every_list_widget_has_the_template() {
    assert_reactive_fixture!(
        "lists/grid_view",
        Widget::from(
            grid_view().template(ItemTemplate::new(path("photos")).item_as("photo"), |vars| {
                bound_text(vars.item_field("title"))
            },)
        )
    );
    assert_reactive_fixture!(
        "lists/page_view",
        Widget::from(
            page_view().template(
                ItemTemplate::new(path("slides"))
                    .item_as("slide")
                    .index_as("p"),
                |vars| bound_text(vars.item_field("title")),
            )
        )
    );
    assert_reactive_fixture!(
        "lists/sliver_list",
        Widget::from(
            sliver_list().template(ItemTemplate::new(path("rows")), |vars| {
                bound_text(vars.item_field("name"))
            })
        )
    );
    assert_reactive_fixture!(
        "lists/sliver_grid",
        Widget::from(
            sliver_grid().template(ItemTemplate::new(path("rows")).item_as("cell"), |vars| {
                bound_text(vars.item_field("name"))
            },)
        )
    );
}

#[test]
fn per_item_checkbox_with_limit_of_three() {
    let selected = path("selected");
    let options = Data::list(
        [("One", 1), ("Two", 2), ("Three", 3), ("Four", 4)]
            .map(|(name, id)| Data::object([("id", Data::Int(id)), ("name", Data::from(name))])),
    );
    let screen = column()
        .state(
            StateBlock::new()
                .with("options", options)
                .with("selected", Data::List(vec![])),
        )
        .children([
            expr_text(Expr::length(selected.clone()) + " of 3 selected"),
            Widget::from(list_view().shrink_wrap(true).template(
                ItemTemplate::new(path("options")).item_as("option"),
                |vars| {
                    let in_list = vars.in_list(selected.clone());
                    let enabled = (vars.count(selected.clone()).lt(3)).or(in_list.clone());
                    Widget::from(
                        list_tile()
                            .title(bound_text(vars.item_field("name")))
                            .trailing(Widget::from(
                                check_box().value(in_list).enabled(enabled).on_changed(
                                    Action::from(
                                        toggle_item("selected").value(vars.item_field("id")),
                                    ),
                                ),
                            )),
                    )
                },
            )),
        ]);
    assert_reactive_fixture!("lists/selection_limit", Widget::from(screen.clone()));
    assert!(
        screen.validate().is_empty(),
        "{}",
        messages(&screen.validate())
    );
}

#[test]
fn in_list_and_count_print_the_contract_expressions() {
    let list = list_view().template(ItemTemplate::new(path("xs")).item_as("x"), |vars| {
        assert_eq!(vars.in_list(path("sel")).to_source(), "contains(sel, x.id)");
        assert_eq!(vars.count(path("sel")).to_source(), "length(sel)");
        Widget::from(text("t"))
    });
    assert!(list.item_template.is_some());
}

#[test]
fn invalid_as_and_index_as_are_reported() {
    for bad in ["1x", "a b", "a.b", "true", "null", "é"] {
        let list = list_view().item_name(bad).index_name("ok");
        let issues = list.validate();
        assert_eq!(issues.len(), 1, "{bad}: {}", messages(&issues));
        assert!(issues[0].path.ends_with("as"), "{}", messages(&issues));

        let list = list_view().item_name("ok").index_name(bad);
        let issues = list.validate();
        assert_eq!(issues.len(), 1, "{bad}: {}", messages(&issues));
        assert!(issues[0].path.ends_with("indexAs"), "{}", messages(&issues));
        // the message names the rule, never the text (`é` is not part of any rule text)
        assert!(!issues[0].message.contains('é'));
    }
    for good in ["todo", "_x", "A1", "item", "index"] {
        assert!(list_view()
            .item_name(good)
            .index_name(good)
            .validate()
            .is_empty());
    }
}

#[test]
fn validation_reaches_the_template_and_the_items_binding() {
    let list = list_view().template(ItemTemplate::new(path("todos")), |vars| {
        // `item` with an invalid field name is reported inside the template
        Widget::from(text("x").visible(Expr::from(vars.item().field("bad name"))))
    });
    assert!(!list.validate().is_empty());
}
