//! Envelope, binding and root fixtures (`fixtures_reactive/envelope`, `fixtures_reactive/bindings`).
//! Actions, lists and expressions have their own files; the inventory (`inventory_state.rs`)
//! ties every item to one of these fixtures.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;

fn path(name: &str) -> StatePath {
    StatePath::key(name)
}

fn label(data: impl Into<Dyn<String>>) -> Widget {
    Widget::from(text(data))
}

fn assert_clean(widget: &Widget) {
    let issues = widget.validate();
    assert!(issues.is_empty(), "{issues:?}");
}

// ---------------------------------------------------------------- visible

#[test]
fn envelope_visible_literal() {
    let w = Widget::from(text("hi").visible(false));
    assert_reactive_fixture!("envelope/visible_literal", w);
    assert_clean(&w);
}

#[test]
fn envelope_visible_state() {
    let w = Widget::from(text("hi").visible(path("ready")));
    assert_reactive_fixture!("envelope/visible_state", w);
    assert_clean(&w);
}

#[test]
fn envelope_visible_expr() {
    let w = Widget::from(text("hi").visible(Expr::length(path("items")).gt(0)));
    assert_reactive_fixture!("envelope/visible_expr", w);
    assert_clean(&w);
}

#[test]
fn envelope_visible_native_visibility() {
    let w = Widget::from(
        visibility()
            .visible(Expr::from(path("a")).and(path("b")))
            .child(label("x")),
    );
    assert_reactive_fixture!("envelope/visible_native_visibility", w);
    assert_clean(&w);
}

#[test]
fn envelope_visible_native_sliver_visibility() {
    let w = Widget::from(sliver_visibility(label("x")).visible(path("show")));
    assert_reactive_fixture!("envelope/visible_native_sliver_visibility", w);
    assert_clean(&w);
}

// ---------------------------------------------------------------- enabled

#[test]
fn envelope_enabled_literal() {
    let w = Widget::from(elevated_button().child(label("Go")).enabled(false));
    assert_reactive_fixture!("envelope/enabled_literal", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_state() {
    let w = Widget::from(elevated_button().child(label("Go")).enabled(path("ready")));
    assert_reactive_fixture!("envelope/enabled_state", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_expr() {
    let w = Widget::from(
        elevated_button()
            .child(label("Go"))
            .enabled(!Expr::from(path("loading"))),
    );
    assert_reactive_fixture!("envelope/enabled_expr", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_text_field() {
    let w = Widget::from(text_field().enabled(path("editable")));
    assert_reactive_fixture!("envelope/enabled_native_text_field", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_text_form_field() {
    let w = Widget::from(text_form_field().enabled(path("editable")));
    assert_reactive_fixture!("envelope/enabled_native_text_form_field", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_dropdown_menu() {
    let w = Widget::from(dropdown_menu().enabled(path("editable")));
    assert_reactive_fixture!("envelope/enabled_native_dropdown_menu", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_list_tile() {
    let w = Widget::from(list_tile().title(label("t")).enabled(path("editable")));
    assert_reactive_fixture!("envelope/enabled_native_list_tile", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_radio() {
    let w = Widget::from(
        radio()
            .value(serde_json::json!("a"))
            .enabled(path("editable")),
    );
    assert_reactive_fixture!("envelope/enabled_native_radio", w);
    assert_clean(&w);
}

#[test]
fn envelope_enabled_native_backdrop_filter() {
    let w = Widget::from(
        backdrop_filter(ImageFilter::new(ImageFilterType::Blur).sigma_x(4.0))
            .enabled(path("blur"))
            .child(label("x")),
    );
    assert_reactive_fixture!("envelope/enabled_native_backdrop_filter", w);
    assert_clean(&w);
}

// ---------------------------------------------------------------- bind

#[test]
fn envelope_bind_check_box() {
    let w = Widget::from(check_box().bind(BindKey::screen("accepted")));
    assert_reactive_fixture!("envelope/bind_check_box", w);
    assert_clean(&w);
}

#[test]
fn envelope_bind_switch() {
    let w = Widget::from(switch().bind(BindKey::screen("on")));
    assert_reactive_fixture!("envelope/bind_switch", w);
    assert_clean(&w);
}

/// `Slider::bound(key)` writes `bind` without `value` (the runtime takes the value from the state).
#[test]
fn envelope_bind_slider() {
    let w = Widget::from(Slider::bound("volume").min(0.0).max(10.0));
    assert_reactive_fixture!("envelope/bind_slider", w);
    assert_clean(&w);
    assert_eq!(
        serde_json::to_value(&w).expect("serializes"),
        serde_json::json!({"type": "slider", "bind": "volume", "min": 0.0, "max": 10.0})
    );
}

#[test]
fn slider_needs_exactly_one_of_value_and_bind() {
    // `new(value)` keeps working and `bind` stays consistent with the other widgets
    assert!(Widget::from(Slider::new(0.5)).validate().is_empty());
    assert!(Slider::bound("volume").validate().is_empty());
    assert!(Slider::bound("global.volume").validate().is_empty());
    let both = Slider::new(0.5).bind("volume");
    let issues = both.validate();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].message.contains("overrides `value`"));
    // the verifier of the serialized screen agrees with the typed rule
    let json = serde_json::to_value(Widget::from(both)).expect("serializes");
    let report = derust::sduix::stacx::v1::state::check_screen(&json);
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    // neither of them
    let neither = Slider::bound("volume");
    let mut empty = neither.clone();
    empty.envelope.bind = None;
    assert_eq!(empty.validate().len(), 1);
    // value given by a binding is still a value
    assert!(Slider::new(StatePath::key("level")).validate().is_empty());
}

#[test]
fn envelope_bind_radio_group() {
    let w = Widget::from(
        radio_group()
            .bind(BindKey::screen("plan"))
            .child(Widget::from(radio().value(serde_json::json!("free")))),
    );
    assert_reactive_fixture!("envelope/bind_radio_group", w);
    assert_clean(&w);
}

#[test]
fn envelope_bind_text_field() {
    let w = Widget::from(text_field().bind(BindKey::screen("name")));
    assert_reactive_fixture!("envelope/bind_text_field", w);
    assert_clean(&w);
}

#[test]
fn envelope_bind_text_form_field() {
    let w = Widget::from(text_form_field().bind(BindKey::screen("email")));
    assert_reactive_fixture!("envelope/bind_text_form_field", w);
    assert_clean(&w);
}

#[test]
fn envelope_bind_dropdown_menu() {
    let w = Widget::from(dropdown_menu().bind(BindKey::screen("country")));
    assert_reactive_fixture!("envelope/bind_dropdown_menu", w);
    assert_clean(&w);
}

#[test]
fn envelope_bind_global_key() {
    let w = Widget::from(text_field().bind(BindKey::global("userName")));
    assert_reactive_fixture!("envelope/bind_global", w);
    assert_clean(&w);
}

// ---------------------------------------------------------------- state declarations

#[test]
fn envelope_state_block_with_every_data_kind() {
    let w = Widget::from(
        column().state(
            StateBlock::new()
                .with("nothing", Data::Null)
                .with("flag", true)
                .with("count", 3)
                .with("ratio", 1.5)
                .with("name", "Ana")
                .with("tags", Data::list(["a", "b"]))
                .with(
                    "user",
                    Data::object([
                        ("name", Data::from("Ana")),
                        ("langs", Data::list(["pt", "en"])),
                    ]),
                ),
        ),
    );
    assert_reactive_fixture!("envelope/state_block", w);
    assert_clean(&w);
}

#[test]
fn envelope_state_value_with_binding() {
    let w = Widget::from(
        column().state(
            StateBlock::new()
                .with("price", 4)
                .with("total", Expr::from(path("price")) * 2)
                .with("owner", StatePath::global("userName"))
                .with("from_params", StatePath::params().field("id")),
        ),
    );
    assert_reactive_fixture!("envelope/state_binding_values", w);
    assert_clean(&w);
}

#[test]
fn envelope_global_state_block() {
    let w = Widget::from(
        column().global_state(
            GlobalStateBlock::new()
                .with("plain", GlobalEntry::value("Ana"))
                .with("persisted", GlobalEntry::declared("light").persist(true))
                .with("secret", GlobalEntry::declared("").secure(true))
                .with(
                    "secret_persisted",
                    GlobalEntry::declared("").persist(true).secure(true),
                )
                .with("memory_only", GlobalEntry::declared(0).persist(false)),
        ),
    );
    assert_reactive_fixture!("envelope/global_state_block", w);
    assert_clean(&w);
}

#[test]
fn envelope_on_init_single_action() {
    let w = Widget::from(
        column()
            .state(StateBlock::new().with("ready", false))
            .on_init(Action::from(set_state("ready", true))),
    );
    assert_reactive_fixture!("envelope/on_init_single", w);
    assert_clean(&w);
}

#[test]
fn envelope_on_init_list_is_the_actions_in_order() {
    let w = Widget::from(column().state(StateBlock::new().with("n", 0)).on_init(vec![
        Action::from(increment_state("n")),
        Action::from(set_state("n", 5)),
    ]));
    assert_reactive_fixture!("envelope/on_init_list", w);
    assert_clean(&w);
}

#[test]
fn envelope_properties_are_written_right_after_the_type() {
    let w = Widget::from(
        column()
            .visible(path("v"))
            .enabled(path("e"))
            .state(StateBlock::new().with("v", true))
            .on_init(Action::from(none_action())),
    );
    assert_reactive_fixture!("envelope/all_properties", w);
}

// ---------------------------------------------------------------- action condition

#[test]
fn envelope_condition_on_existing_action() {
    let navigate = NavigateAction::new()
        .navigation_style(NavigationStyle::Push)
        .route_name("/next")
        .condition(Expr::from(path("ready")).and(!Expr::from(path("busy"))));
    assert_reactive_fixture!("envelope/condition_navigate", Action::from(navigate));
}

#[test]
fn envelope_condition_literal_on_action() {
    let a = set_state("x", 1).condition(true);
    assert_reactive_fixture!("envelope/condition_literal", Action::from(a));
}

// ---------------------------------------------------------------- bindings

#[test]
fn binding_state_path_forms() {
    let w = Widget::from(column().children([
        label(Binding::State(path("title"))),
        label(Binding::State(path("user").field("address").field("city"))),
        label(Binding::State(path("items").index(0).field("name"))),
    ]));
    assert_reactive_fixture!("bindings/state_paths", w);
    assert_clean(&w);
}

#[test]
fn binding_expr_form() {
    let w = label(Binding::Expr(
        Expr::text("Total: ") + Expr::from(path("price")) * path("qty"),
    ));
    assert_reactive_fixture!("bindings/expr", w);
    assert_clean(&w);
}

#[test]
fn binding_in_a_non_text_property() {
    let w = Widget::from(
        sized_box()
            .height(Expr::from(path("base")) + 4)
            .width(path("w")),
    );
    assert_reactive_fixture!("bindings/numeric_property", w);
    assert_clean(&w);
}

#[test]
fn binding_root_key() {
    let w = label(Binding::State(path("total")));
    assert_reactive_fixture!("bindings/root_key", w);
}

#[test]
fn binding_root_global() {
    let w = label(Binding::State(StatePath::global("token")));
    assert_reactive_fixture!("bindings/root_global", w);
}

#[test]
fn binding_root_params() {
    let w = label(Binding::State(StatePath::params().field("name")));
    assert_reactive_fixture!("bindings/root_params", w);
}

#[test]
fn binding_root_result() {
    let w = label(Binding::State(StatePath::result().field("qty")));
    assert_reactive_fixture!("bindings/root_result", w);
}

#[test]
fn binding_root_response() {
    let w = label(Binding::State(
        StatePath::response().field("body").field("items"),
    ));
    assert_reactive_fixture!("bindings/root_response", w);
}

#[test]
fn binding_root_event() {
    let w = label(Binding::State(StatePath::event().field("value")));
    assert_reactive_fixture!("bindings/root_event", w);
}

#[test]
fn binding_root_item_and_index() {
    let w = Widget::from(
        list_view().template(ItemTemplate::new(path("rows")), |vars| {
            label(Binding::Expr(
                Expr::from(vars.index()) + Expr::text(": ") + vars.item_field("name"),
            ))
        }),
    );
    assert_reactive_fixture!("bindings/root_item_index", w);
    assert_clean(&w);
}

#[test]
fn global_prefix_is_always_dotted() {
    let json = serde_json::to_value(Binding::State(StatePath::global("user").field("name")))
        .expect("serializes");
    assert_eq!(json, serde_json::json!({"$state": "global.user.name"}));
    let expr = Expr::from(StatePath::global("user"));
    assert_eq!(expr.to_source(), "global.user");
}
