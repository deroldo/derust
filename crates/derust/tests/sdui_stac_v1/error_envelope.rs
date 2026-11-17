//! The error envelopes of `examples/sdui_reactive` (same source file) against
//! `fixtures_reactive/error_envelope`. These fixtures are the source of truth for the envelope
//! serialization (the front keeps a copy).
use super::assert_reactive_fixture;
use axum::http::StatusCode;
use derust::httpx::stac_error::StacErrorResponseBuilder;
use derust::httpx::HttpResponse;
use serde_json::Value;

#[path = "../../../../examples/sdui_reactive/src/stac_error.rs"]
mod stac_error;

fn body(builder: StacErrorResponseBuilder) -> Value {
    let response = builder.build().expect("valid envelope");
    serde_json::from_str(&response.response_body().unwrap()).unwrap()
}

#[test]
fn minimal_envelope() {
    assert_reactive_fixture!("error_envelope/minimal", body(stac_error::minimal()));
}

#[test]
fn screen_only_envelope() {
    assert_reactive_fixture!(
        "error_envelope/screen_only",
        body(stac_error::screen_only())
    );
}

#[test]
fn action_only_envelope() {
    assert_reactive_fixture!(
        "error_envelope/action_only",
        body(stac_error::action_only())
    );
}

#[test]
fn full_envelope_with_recovery() {
    assert_reactive_fixture!("error_envelope/full", body(stac_error::full()));
}

#[test]
fn session_cycle_envelope() {
    let response = stac_error::session_cycle().build().unwrap();
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_reactive_fixture!(
        "error_envelope/session_cycle",
        body(stac_error::session_cycle())
    );
}
