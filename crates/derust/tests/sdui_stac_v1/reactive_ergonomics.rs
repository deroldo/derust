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
