//! Coherence spike: `impl<T: Plain> IntoReactive<T> for T` must coexist with the impls for
//! `Expr`/`Binding`/`StatePath` and the compatibility impls, and inference must work.
use derust::sduix::stacx::v1::state::{Binding, Expr, IntoReactive, Reactive, StatePath};
use serde_json::json;

#[derive(Default)]
struct Props {
    spacing: Option<Reactive<f64>>,
    data: Option<Reactive<String>>,
    flag: Option<Reactive<bool>>,
    count: Option<Reactive<i64>>,
    tags: Option<Reactive<Vec<String>>>,
}

impl Props {
    fn new() -> Self {
        Self::default()
    }

    fn spacing(mut self, v: impl IntoReactive<f64>) -> Self {
        self.spacing = Some(v.into_reactive());
        self
    }

    fn data(mut self, v: impl IntoReactive<String>) -> Self {
        self.data = Some(v.into_reactive());
        self
    }

    fn flag(mut self, v: impl IntoReactive<bool>) -> Self {
        self.flag = Some(v.into_reactive());
        self
    }

    fn count(mut self, v: impl IntoReactive<i64>) -> Self {
        self.count = Some(v.into_reactive());
        self
    }

    fn tags(mut self, v: impl IntoReactive<Vec<String>>) -> Self {
        self.tags = Some(v.into_reactive());
        self
    }
}

fn json_of<T: serde::Serialize>(v: &Option<Reactive<T>>) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

#[test]
fn spacing_accepts_numbers_and_bindings() {
    assert_eq!(json_of(&Props::new().spacing(8).spacing), json!(8.0));
    assert_eq!(json_of(&Props::new().spacing(8.0).spacing), json!(8.0));
    assert_eq!(json_of(&Props::new().spacing(8_f32).spacing), json!(8.0));
    assert_eq!(json_of(&Props::new().spacing(8_u32).spacing), json!(8.0));
    let expr = Expr::from(StatePath::key("n")) + 1;
    assert_eq!(
        json_of(&Props::new().spacing(expr).spacing),
        json!({"$expr": "n + 1"})
    );
    assert_eq!(
        json_of(&Props::new().spacing(StatePath::key("gap")).spacing),
        json!({"$state": "gap"})
    );
    assert_eq!(
        json_of(
            &Props::new()
                .spacing(Binding::from(StatePath::global("g")))
                .spacing
        ),
        json!({"$state": "global.g"})
    );
}

#[test]
fn text_accepts_str_string_and_bindings() {
    assert_eq!(json_of(&Props::new().data("a").data), json!("a"));
    assert_eq!(
        json_of(&Props::new().data(String::from("b")).data),
        json!("b")
    );
    assert_eq!(
        json_of(&Props::new().data(Expr::text("c")).data),
        json!({"$expr": "'c'"})
    );
    assert_eq!(
        json_of(&Props::new().data(StatePath::key("s")).data),
        json!({"$state": "s"})
    );
}

#[test]
fn other_types_infer() {
    assert_eq!(json_of(&Props::new().flag(true).flag), json!(true));
    assert_eq!(
        json_of(
            &Props::new()
                .flag(Expr::from(StatePath::key("a")).gt(0))
                .flag
        ),
        json!({"$expr": "a > 0"})
    );
    assert_eq!(json_of(&Props::new().count(3).count), json!(3));
    assert_eq!(
        json_of(&Props::new().tags(vec!["a", "b"]).tags),
        json!(["a", "b"])
    );
    assert_eq!(
        json_of(&Props::new().tags(vec![String::from("a")]).tags),
        json!(["a"])
    );
    assert_eq!(
        json_of(&Props::new().tags(Reactive::bind(StatePath::key("t"))).tags),
        json!({"$state": "t"})
    );
}

// --- against the real generated types (T3) ---

mod real_types {
    use super::*;
    use derust::sduix::stacx::common::Validate;
    use derust::sduix::stacx::v1::prelude::*;

    fn count(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    #[test]
    fn padding_and_visible_by_expression() {
        let widget = Container::new()
            .padding(EdgeInsets::all(8.0))
            .visible(Expr::from(StatePath::key("items").field("length")).gt(0));
        assert_eq!(
            serde_json::to_value(&widget).unwrap(),
            json!({
                "type": "container",
                "visible": {"$expr": "items.length > 0"},
                "padding": {"left": 8.0, "top": 8.0, "right": 8.0, "bottom": 8.0}
            })
        );
    }

    #[test]
    fn padding_itself_can_be_bound_and_insets_accept_paths() {
        let widget = Container::new().padding(StatePath::key("pad"));
        assert_eq!(
            serde_json::to_value(&widget).unwrap()["padding"],
            json!({"$state": "pad"})
        );
        let insets = EdgeInsets::all(StatePath::key("gap"));
        assert_eq!(
            serde_json::to_value(&insets).unwrap()["top"],
            json!({"$state": "gap"})
        );
    }

    #[test]
    fn spacing_by_state_path_and_numbers() {
        let bound = Column::new().spacing(StatePath::key("gap"));
        assert_eq!(
            serde_json::to_value(&bound).unwrap(),
            json!({"type": "column", "spacing": {"$state": "gap"}})
        );
        let literal = Column::new().spacing(8);
        assert_eq!(
            serde_json::to_value(&literal).unwrap()["spacing"],
            json!(8.0)
        );
    }

    #[test]
    fn native_visible_is_gone_and_serialized_once() {
        let widget = Visibility::new().visible(true);
        let text = serde_json::to_string(&widget).unwrap();
        assert_eq!(text, r#"{"type":"visibility","visible":true}"#);

        let by_expr = Visibility::new().visible(Expr::from(StatePath::key("on")));
        let text = serde_json::to_string(&by_expr).unwrap();
        assert_eq!(count(&text, "\"visible\""), 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&text).unwrap()["visible"],
            json!({"$expr": "on"})
        );
    }

    #[test]
    fn text_field_bind_and_enabled() {
        let field = TextField::new()
            .bind(BindKey::screen("name"))
            .enabled(Expr::from(StatePath::key("editing")))
            .read_only(false);
        assert_eq!(
            serde_json::to_value(&field).unwrap(),
            json!({
                "type": "textField",
                "enabled": {"$expr": "editing"},
                "bind": "name",
                "readOnly": false
            })
        );
        let text = serde_json::to_string(&field).unwrap();
        assert_eq!(count(&text, "\"enabled\""), 1);
    }

    #[test]
    fn every_widget_with_a_native_flag_writes_the_key_once() {
        let cases: Vec<(&str, String)> = vec![
            (
                "visible",
                serde_json::to_string(&Visibility::new().visible(false)).unwrap(),
            ),
            (
                "visible",
                serde_json::to_string(&SliverVisibility::new(Text::new("a")).visible(false))
                    .unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(
                    &BackdropFilter::new(ImageFilter::new(ImageFilterType::Blur)).enabled(false),
                )
                .unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(&DropdownMenu::new().enabled(false)).unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(&ListTile::new().enabled(false)).unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(&Radio::new().enabled(false)).unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(&TextField::new().enabled(false)).unwrap(),
            ),
            (
                "enabled",
                serde_json::to_string(&TextFormField::new().enabled(false)).unwrap(),
            ),
        ];
        for (key, text) in cases {
            assert_eq!(count(&text, &format!("\"{key}\"")), 1, "{text}");
        }
    }

    #[test]
    fn full_envelope_follows_the_discriminator() {
        let widget = Container::new()
            .visible(true)
            .enabled(Expr::from(StatePath::key("ok")))
            .bind(BindKey::global("name"))
            .state(StateBlock::new().with("n", 0))
            .global_state(GlobalStateBlock::new().with("token", GlobalEntry::value("t")))
            .on_init(vec![Action::from(navigate().route_name("/home"))])
            .alignment(Alignment::Center);
        let text = serde_json::to_string(&widget).unwrap();
        assert!(
            text.starts_with(
                r#"{"type":"container","visible":true,"enabled":{"$expr":"ok"},"bind":"global.name","state":{"n":0},"globalState":{"token":"t"},"onInit":[{"actionType":"navigate","routeName":"/home"}],"alignment":"center""#
            ),
            "{text}"
        );
        assert!(widget.validate().is_empty());
    }

    #[test]
    fn action_condition_is_serialized_after_the_discriminator() {
        let action = navigate()
            .route_name("/x")
            .condition(Expr::from(StatePath::key("ok")).and(Expr::from(StatePath::key("ready"))));
        assert_eq!(
            serde_json::to_string(&action).unwrap(),
            r#"{"actionType":"navigate","condition":{"$expr":"ok && ready"},"routeName":"/x"}"#
        );
        let plain = navigate().route_name("/x").condition(false);
        assert_eq!(
            serde_json::to_value(&plain).unwrap()["condition"],
            json!(false)
        );
    }

    #[test]
    fn snack_bar_action_is_nested_only_and_has_no_condition() {
        let text = serde_json::to_string(&SnackBarAction::new("Undo")).unwrap();
        assert!(!text.contains("condition"), "{text}");
    }

    #[test]
    fn validation_reaches_envelope_and_bound_properties() {
        let widget = Container::new()
            .visible(Expr::num(f64::NAN))
            .padding(Expr::num(f64::INFINITY));
        let paths: Vec<_> = widget.validate().into_iter().map(|i| i.path).collect();
        assert!(paths.contains(&"$.visible.$expr".to_string()), "{paths:?}");
        assert!(paths.contains(&"$.padding.$expr".to_string()), "{paths:?}");
    }

    #[test]
    fn widget_enum_keeps_the_envelope() {
        let widget = Widget::from(Text::new("a").visible(false));
        assert_eq!(
            serde_json::to_value(&widget).unwrap(),
            json!({"type": "text", "visible": false, "data": "a"})
        );
    }

    #[test]
    fn dyn_string_accepts_expressions_and_paths() {
        let by_expr = Text::new(Expr::from(StatePath::key("a")) + Expr::text("!"));
        assert_eq!(
            serde_json::to_value(&by_expr).unwrap()["data"],
            json!({"$expr": "a + '!'"})
        );
        let by_path = Text::new(StatePath::key("title"));
        assert_eq!(
            serde_json::to_value(&by_path).unwrap()["data"],
            json!({"$state": "title"})
        );
    }
}
