//! `Recovery` / `Retry` against `fixtures_reactive/recovery`.
use super::assert_reactive_fixture;
use derust::sduix::stacx::v1::prelude::*;
use serde_json::json;

fn refresh() -> NetworkRequest {
    NetworkRequest::new("https://example.com/auth/refresh")
        .method(Method::Post)
        .on_error(Action::from(recovery_outcome(RecoveryResult::Failure)))
        .on_success(Action::from(recovery_outcome(RecoveryResult::Success)))
}

fn text_of(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn minimal_recovery() {
    let recovery = Recovery::new().recover(Action::from(refresh()));
    assert!(recovery.validate().is_empty());
    assert_reactive_fixture!("recovery/minimal", recovery);
}

#[test]
fn full_recovery() {
    let recovery = Recovery::new()
        .key("session")
        .recover(vec![Action::from(refresh())])
        .retry(
            Retry::new()
                .header("Authorization", StatePath::key("token"))
                .query("v", 2)
                .body("a", "b")
                .remove_body_key("old")
                .remove_header("X-Stale"),
        )
        .on_failure(Action::from(set_state("failed", true)))
        .on_unavailable(Action::from(set_state("down", true)))
        .timeout_ms(5000);
    assert!(recovery.validate().is_empty(), "{:?}", recovery.validate());
    assert_reactive_fixture!("recovery/full", recovery);
}

#[test]
fn no_coalesce_serializes_null_key() {
    let recovery = Recovery::new()
        .no_coalesce()
        .recover(Action::from(refresh()));
    let value = serde_json::to_value(&recovery).unwrap();
    assert_eq!(value["key"], json!(null));
    assert!(value.as_object().unwrap().contains_key("key"));
    assert_reactive_fixture!("recovery/no_coalesce", recovery);
}

#[test]
fn unset_key_is_omitted_and_retry_has_no_url_or_method() {
    let recovery = Recovery::new()
        .recover(Action::from(refresh()))
        .retry(Retry::new().header("A", "b"));
    let value = serde_json::to_value(&recovery).unwrap();
    assert!(!value.as_object().unwrap().contains_key("key"));
    let retry = value["retry"].as_object().unwrap();
    assert!(!retry.contains_key("url") && !retry.contains_key("method"));
    assert!(!value["retry"]["overrides"]
        .as_object()
        .unwrap()
        .contains_key("body"));
}

#[test]
fn empty_retry_is_an_empty_object() {
    let recovery = Recovery::new()
        .recover(Action::from(refresh()))
        .retry(Retry::new());
    assert_eq!(serde_json::to_value(&recovery).unwrap()["retry"], json!({}));
}

#[test]
fn invalid_recoveries_are_rejected() {
    let ok = || Recovery::new().recover(Action::from(refresh()));
    let cases: Vec<(Recovery, &str)> = vec![
        (Recovery::new(), "`recover` is required"),
        (Recovery::new().recover(Vec::<Action>::new()), "empty list"),
        (ok().key(""), "key is empty"),
        (ok().key("a".repeat(129)), "longer than"),
        (ok().key("a\nb"), "control"),
        (ok().timeout_ms(0), "timeoutMs"),
        (ok().timeout_ms(600_001), "timeoutMs"),
        (
            ok().retry(Retry::new().header("bad name", "x")),
            "HTTP token",
        ),
        (ok().retry(Retry::new().remove_header("")), "HTTP token"),
        (
            ok().on_failure(Action::from(set_state("a", f64::NAN))),
            "onFailure",
        ),
    ];
    for (recovery, expected) in cases {
        let text = text_of(&recovery.validate());
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn missing_recover_fails_serialization() {
    assert!(serde_json::to_value(Recovery::new()).is_err());
}
