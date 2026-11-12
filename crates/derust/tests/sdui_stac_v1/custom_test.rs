//! Extension points for applications (this test crate plays the role of a derust user):
//! reusable compositions of standard widgets and custom widgets/actions registered in the
//! Flutter app.
use derust::sduix::stacx::common::{CustomAction, CustomWidget, Path};
use derust::sduix::stacx::v1::prelude::*;
use serde::Serialize;
use serde_json::json;

// ------------------------------------------------------------------ composition

/// A screen template owned by the application: only standard Stac widgets in the JSON.
struct StandardScreen {
    title: String,
    body: Widget,
}

impl StandardScreen {
    fn new(title: impl Into<String>, body: impl Into<Widget>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

impl From<StandardScreen> for Widget {
    fn from(screen: StandardScreen) -> Self {
        Widget::from(
            scaffold()
                .app_bar(app_bar().title(text(screen.title)))
                .body(screen.body),
        )
    }
}

#[test]
fn compositions_are_plain_widgets() {
    let screen = Widget::from(StandardScreen::new("Home", text("hello")));
    assert_eq!(
        serde_json::to_value(&screen).unwrap(),
        json!({
            "type": "scaffold",
            "appBar": {"type": "appBar", "title": {"type": "text", "data": "Home"}},
            "body": {"type": "text", "data": "hello"}
        })
    );
    assert!(screen.validate().is_empty());
}

#[test]
fn compositions_nest_like_any_widget() {
    let screen = Widget::from(column().children([Widget::from(StandardScreen::new(
        "Inner",
        container().padding(EdgeInsets::all(f64::NAN)),
    ))]));
    let issues = screen.validate();
    // one issue per side of the padding
    assert_eq!(issues.len(), 4);
    assert!(issues
        .iter()
        .all(|issue| issue.path.starts_with("$.children[0].body.padding.")));
}

// ------------------------------------------------------------------ custom widgets/actions

/// A widget implemented by a custom parser in the Flutter app.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateText {
    state_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fallback: Option<Widget>,
}

impl Validate for StateText {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if self.state_key.is_empty() {
            issues.push(ValidationIssue::new(
                &path.key("stateKey"),
                "must not be empty",
            ));
        }
        self.fallback.validate_at(&path.key("fallback"), issues);
    }
}

impl CustomWidget for StateText {
    const TYPE_NAME: &'static str = "stateText";
}

/// An action implemented by a custom parser in the Flutter app.
#[derive(Debug, Serialize)]
struct SetState {
    key: String,
    value: String,
}

impl Validate for SetState {
    fn validate_at(&self, _path: &Path<'_>, _issues: &mut Vec<ValidationIssue>) {}
}

impl CustomAction for SetState {
    const ACTION_TYPE: &'static str = "setState";
}

fn state_text(key: &str) -> StateText {
    StateText {
        state_key: key.to_string(),
        fallback: None,
    }
}

#[test]
fn custom_widgets_and_actions_mix_with_standard_ones() {
    let screen = Widget::from(
        column().children([
            Widget::from(state_text("user.name")),
            Widget::from(
                text_button()
                    .on_pressed(SetState {
                        key: "user.name".into(),
                        value: "Ada".into(),
                    })
                    .child(text("Set name")),
            ),
        ]),
    );
    assert_eq!(
        serde_json::to_value(&screen).unwrap(),
        json!({
            "type": "column",
            "children": [
                {"type": "stateText", "stateKey": "user.name"},
                {
                    "type": "textButton",
                    "onPressed": {"actionType": "setState", "key": "user.name", "value": "Ada"},
                    "child": {"type": "text", "data": "Set name"}
                }
            ]
        })
    );
    assert!(screen.validate().is_empty());
}

#[test]
fn custom_discriminators_are_exposed() {
    assert_eq!(Widget::from(state_text("a")).type_name(), "stateText");
    let action = Action::from(SetState {
        key: "a".into(),
        value: "b".into(),
    });
    assert_eq!(action.type_name(), "setState");
}

#[test]
fn standard_widgets_nest_inside_custom_ones_and_are_validated() {
    let widget = Widget::from(StateText {
        state_key: String::new(),
        fallback: Some(Widget::from(
            container().padding(EdgeInsets::all(f64::INFINITY)),
        )),
    });
    let value = serde_json::to_value(&widget).unwrap();
    assert_eq!(value["fallback"]["type"], "container");

    let paths: Vec<_> = widget.validate().into_iter().map(|i| i.path).collect();
    assert!(paths.contains(&"$.stateKey".to_string()), "{paths:?}");
    assert!(
        paths.iter().any(|p| p.starts_with("$.fallback.padding")),
        "{paths:?}"
    );
}

#[derive(Debug, Serialize)]
struct Clashing {
    r#type: String,
}

impl Validate for Clashing {
    fn validate_at(&self, _path: &Path<'_>, _issues: &mut Vec<ValidationIssue>) {}
}

impl CustomWidget for Clashing {
    const TYPE_NAME: &'static str = "clashing";
}

#[derive(Debug, Serialize)]
struct NotAnObject(u32);

impl Validate for NotAnObject {
    fn validate_at(&self, _path: &Path<'_>, _issues: &mut Vec<ValidationIssue>) {}
}

impl CustomWidget for NotAnObject {
    const TYPE_NAME: &'static str = "notAnObject";
}

#[test]
fn custom_field_clashing_with_discriminator_is_rejected() {
    let widget = Widget::from(column().children([Widget::from(Clashing { r#type: "x".into() })]));
    let issues = widget.validate();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, "$.children[0]");
    assert!(serde_json::to_value(&widget).is_err());
}

#[test]
fn custom_node_that_is_not_an_object_is_rejected() {
    let widget = Widget::from(NotAnObject(1));
    assert_eq!(widget.validate().len(), 1);
    assert!(serde_json::to_value(&widget).is_err());
}

#[test]
fn custom_nodes_keep_widget_clone_eq_send_sync() {
    fn assert_send_sync<T: Send + Sync + Clone + PartialEq>() {}
    assert_send_sync::<Widget>();
    assert_send_sync::<Action>();

    let widget = Widget::from(state_text("a"));
    assert_eq!(widget.clone(), widget);
    assert_ne!(widget, Widget::from(state_text("b")));
    assert_ne!(widget, Widget::from(text("a")));
}
