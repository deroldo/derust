//! The screen of `examples/sdui` (same source file) must serialize to the fixture that the real
//! Stac parses in CI (`tools/stac-compat/v1`).
use derust::sduix::stacx::common::Validate;

#[path = "../../../../examples/sdui/src/screen.rs"]
mod screen;

#[test]
fn home_screen_matches_fixture() {
    let screen = screen::home_screen();
    assert!(screen.validate().is_empty());
    crate::assert_fixture!("examples/home_screen", screen);
}
