//! `navigate.routeName`: serialization against `fixtures_reactive/actions/navigate_route_name_*`
//! and the format validation (relative path when the action loads a screen, basic rules otherwise).
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use serde_json::json;

const STYLES_NAMED: [NavigationStyle; 3] = [
    NavigationStyle::PushNamed,
    NavigationStyle::PushReplacementNamed,
    NavigationStyle::PushNamedAndRemoveAll,
];
const STYLES_PUSH: [NavigationStyle; 3] = [
    NavigationStyle::Push,
    NavigationStyle::PushReplacement,
    NavigationStyle::PushAndRemoveAll,
];

fn request() -> NetworkRequest {
    NetworkRequest::new("https://example.com/screens/profile").method(Method::Get)
}

fn screen() -> Widget {
    Widget::from(text("hi"))
}

fn json_of(nav: &NavigateAction) -> serde_json::Value {
    serde_json::to_value(nav).unwrap()
}

fn messages(nav: &NavigateAction) -> Vec<(String, String)> {
    nav.validate()
        .into_iter()
        .map(|i| (i.path, i.message))
        .collect()
}

/// A navigate that loads a screen (`request`) and names the route.
fn with_request(name: &str) -> NavigateAction {
    NavigateAction::new().request(request()).route_name(name)
}

// ---------------------------------------------------------------- serialization

#[test]
fn route_name_with_request() {
    let nav = with_request("/screens/profile?choose=true").navigation_style(NavigationStyle::Push);
    assert_reactive_fixture!(
        "actions/navigate_route_name_request",
        Action::from(nav.clone())
    );
    assert!(nav.validate().is_empty());
}

#[test]
fn route_name_with_widget_json() {
    let nav = NavigateAction::new()
        .widget_json(screen())
        .route_name("/screens/profile");
    assert_reactive_fixture!(
        "actions/navigate_route_name_widget_json",
        Action::from(nav.clone())
    );
    assert!(nav.validate().is_empty());
}

#[test]
fn route_name_with_asset_path() {
    let nav = NavigateAction::new()
        .asset_path("assets/profile.json")
        .route_name("/screens/a/b?x=1&y=2")
        .navigation_style(NavigationStyle::PushReplacement);
    assert_reactive_fixture!(
        "actions/navigate_route_name_asset_path",
        Action::from(nav.clone())
    );
    assert!(nav.validate().is_empty());
}

#[test]
fn route_name_alone_names_a_stac_screen() {
    let nav = NavigateAction::new().route_name("detail_screen");
    assert_reactive_fixture!(
        "actions/navigate_route_name_only",
        Action::from(nav.clone())
    );
    assert!(nav.validate().is_empty());
}

#[test]
fn route_name_is_absent_when_none() {
    let nav = NavigateAction::new().request(request());
    let value = json_of(&nav);
    assert!(value.get("routeName").is_none(), "{value}");
}

#[test]
fn route_name_json_key_and_coexistence_with_request() {
    let value = json_of(&with_request("/screens/profile"));
    assert_eq!(value["routeName"], "/screens/profile");
    assert_eq!(
        value["request"]["url"],
        "https://example.com/screens/profile"
    );
    assert_eq!(value["actionType"], "navigate");
    // `actionType` is always the first key
    let keys: Vec<_> = value.as_object().unwrap().keys().collect();
    assert_eq!(keys[0], "actionType");
    // the fixture keeps the contract shape (both keys at the same level)
    assert!(value.as_object().unwrap().contains_key("request"));
    assert!(value.as_object().unwrap().contains_key("routeName"));
}

#[test]
fn route_name_is_serialized_with_or_without_request() {
    for nav in [
        NavigateAction::new().route_name("/settings"),
        with_request("/screens/x"),
    ] {
        assert!(json_of(&nav).get("routeName").is_some());
    }
}

// ---------------------------------------------------------------- strict rules (loads a screen)

#[test]
fn valid_screen_route_names() {
    for name in [
        "/screens/profile",
        "/screens/profile?choose=true",
        "/screens/a/b?x=1&y=2",
        "/telas/x?id=1",
        "/other/x",
    ] {
        assert!(with_request(name).validate().is_empty(), "{name}");
        assert!(NavigateAction::new()
            .widget_json(screen())
            .route_name(name)
            .validate()
            .is_empty());
        assert!(NavigateAction::new()
            .asset_path("a.json")
            .route_name(name)
            .validate()
            .is_empty());
    }
}

#[test]
fn invalid_screen_route_names() {
    for name in [
        "screens/x",
        "a:b/c",
        "//evil.com/screens/x",
        "https://h/screens/x",
        "http:/screens/x",
        "javascript:alert(1)",
        "/screens/x y",
        "/screens/x\ty",
        "/screens/x\ny",
        "/screens/x\u{0}",
        "/screens/x\u{7f}",
        "/screens/x\\y",
        "/screens/x?u=http://h/",
        "",
    ] {
        let issues = messages(&with_request(name));
        assert_eq!(issues.len(), 1, "{name:?}: {issues:?}");
        assert_eq!(issues[0].0, "$.routeName", "{name:?}");
    }
}

#[test]
fn strict_rules_apply_to_every_push_style_and_to_the_absent_style() {
    for style in STYLES_PUSH {
        let nav = with_request("detail_screen").navigation_style(style);
        assert_eq!(nav.validate().len(), 1, "{style:?}");
    }
    assert_eq!(with_request("detail_screen").validate().len(), 1);
}

#[test]
fn explicit_scheme_host_and_authority_checks() {
    let cases = [
        ("//evil.com/screens/x", "`//`"),
        ("https://h/screens/x", "scheme or host"),
        ("mailto:a@b/screens/x", "scheme or host"),
    ];
    for (name, expected) in cases {
        let issues = messages(&with_request(name));
        assert!(issues[0].1.contains(expected), "{name}: {}", issues[0].1);
    }
}

// ---------------------------------------------------------------- basic rules (everything else)

#[test]
fn named_styles_and_bare_names_keep_the_basic_rules_only() {
    // a Stac screen name in push style without request/widgetJson/assetPath
    for style in STYLES_PUSH {
        let nav = NavigateAction::new()
            .route_name("detail_screen")
            .navigation_style(style);
        assert!(nav.validate().is_empty(), "{style:?}");
    }
    // a Flutter route name in the named styles, even next to a request
    for style in STYLES_NAMED {
        let nav = NavigateAction::new()
            .route_name("/settings")
            .navigation_style(style);
        assert!(nav.validate().is_empty(), "{style:?}");
        let with_req = with_request("/settings").navigation_style(style);
        assert!(with_req.validate().is_empty(), "{style:?}");
    }
    assert!(NavigateAction::new()
        .route_name("/settings")
        .validate()
        .is_empty());
}

#[test]
fn basic_rules_reject_empty_whitespace_and_control() {
    for name in ["", " ", "a b", "/settings\t", "/a\nb", "x\u{1}"] {
        let nav = NavigateAction::new()
            .route_name(name)
            .navigation_style(NavigationStyle::PushNamed);
        let issues = messages(&nav);
        assert_eq!(issues.len(), 1, "{name:?}: {issues:?}");
        assert_eq!(issues[0].0, "$.routeName");
    }
}

// ---------------------------------------------------------------- values only known at runtime

#[test]
fn binding_and_legacy_template_are_not_checked() {
    let by_state = NavigateAction::new()
        .request(request())
        .route_name(StatePath::key("route"));
    assert!(by_state.validate().is_empty());
    let by_expr = NavigateAction::new()
        .request(request())
        .route_name(Expr::from("/screens/") + StatePath::key("id"));
    assert!(by_expr.validate().is_empty());
    let legacy = with_request("{{route}}");
    assert!(legacy.validate().is_empty());
    let legacy_ref = NavigateAction::new()
        .request(request())
        .route_name(StateRef::new("route"));
    assert!(legacy_ref.validate().is_empty());
}

#[test]
fn bound_style_only_applies_the_basic_rules() {
    let nav = NavigateAction::new()
        .request(request())
        .route_name("detail_screen")
        .navigation_style(StatePath::key("style"));
    assert!(nav.validate().is_empty());
}

// ---------------------------------------------------------------- messages and coexistence

#[test]
fn messages_carry_the_path_and_never_the_value() {
    let secret = "other/tok3n-S3CRET";
    let issues = with_request(secret).validate();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, "$.routeName");
    let rendered = format!("{} {}", issues[0].path, issues[0].message);
    assert!(!rendered.contains("S3CRET"), "{rendered}");
    assert!(!rendered.contains("other"), "{rendered}");
    for name in ["x y S3CRET", "https://h/S3CRET", "/screens/S3CRET\\z", ""] {
        for nav in [
            with_request(name),
            NavigateAction::new()
                .route_name(name)
                .navigation_style(NavigationStyle::PushNamed),
        ] {
            for issue in nav.validate() {
                assert!(!issue.message.contains("S3CRET"), "{}", issue.message);
                assert!(!issue.to_string().contains("S3CRET"), "{issue}");
            }
        }
    }
}

#[test]
fn path_is_nested_when_the_action_is_inside_a_widget() {
    let button = elevated_button()
        .child(Widget::from(text("Go")))
        .on_pressed(Action::from(with_request("other/x")));
    let issues = Widget::from(button).validate();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].path.ends_with(".routeName"), "{}", issues[0].path);
}

#[test]
fn coexists_with_result_and_callbacks_rules() {
    // routeName errors are reported alongside, and never replace, the existing navigate rules
    let nav = with_request("other/x")
        .result(Data::Null)
        .on_result(Action::from(none_action()))
        .on_cancel(Action::from(none_action()));
    let paths: Vec<_> = nav.validate().into_iter().map(|i| i.path).collect();
    assert_eq!(paths, vec!["$.routeName", "$.result"]);
    // a valid routeName leaves the result rule untouched
    let ok = with_request("/screens/x")
        .navigation_style(NavigationStyle::PushReplacement)
        .result(Data::Null)
        .on_result(Action::from(none_action()))
        .on_cancel(Action::from(none_action()));
    assert!(ok.validate().is_empty());
}

#[test]
fn json_shape_matches_the_contract() {
    let nav = with_request("/screens/profile");
    assert_eq!(
        json_of(&nav),
        json!({
            "actionType": "navigate",
            "request": {"actionType": "networkRequest", "url": "https://example.com/screens/profile", "method": "get"},
            "routeName": "/screens/profile"
        })
    );
}
