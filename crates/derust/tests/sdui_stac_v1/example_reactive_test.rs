//! The screens of `examples/sdui_reactive` (same source file) must serialize to the fixtures that
//! the derust-stac fork parses in CI (`tools/stac-compat/v1-reactive`) and pass both validation
//! layers; the invalid one must be rejected with a message that locates the problem.
use derust::sduix::stacx::v1::prelude::*;
use derust::sduix::stacx::v1::state::check_screen;

#[path = "../../../../examples/sdui_reactive/src/screen.rs"]
mod screen;

fn assert_valid(screen: &Widget) {
    let issues = screen.validate();
    assert!(issues.is_empty(), "typed validation: {issues:?}");
    let json = serde_json::to_value(screen).expect("serializes");
    let report = check_screen(&json);
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(
        report.warnings.is_empty(),
        "warnings: {:?}",
        report.warnings
    );
}

#[test]
fn form_screen_matches_fixture() {
    let screen = screen::form_screen();
    assert_valid(&screen);
    crate::assert_reactive_fixture!("screens/example_form", screen);
}

#[test]
fn list_screen_matches_fixture() {
    let screen = screen::list_screen();
    assert_valid(&screen);
    crate::assert_reactive_fixture!("screens/example_list", screen);
}

#[test]
fn network_screen_matches_fixture() {
    let screen = screen::network_screen();
    assert_valid(&screen);
    crate::assert_reactive_fixture!("screens/example_network", screen);
}

#[test]
fn navigation_screen_matches_fixture() {
    let screen = screen::navigation_screen();
    assert_valid(&screen);
    crate::assert_reactive_fixture!("screens/example_navigation", screen);
}

#[test]
fn invalid_screen_is_rejected_with_its_location() {
    let screen = screen::invalid_screen();
    assert!(screen.validate().is_empty());
    let json = serde_json::to_value(&screen).expect("serializes");
    let report = check_screen(&json);
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert!(report.errors[0].path.starts_with("$.children[0].onPressed"));
    assert!(report.errors[0].message.contains("missing"));
}
