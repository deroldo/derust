//! Stac error envelopes (`StacErrorResponse`): minimal, screen only, action only, full and a
//! session-expired cycle with a `recovery`.
//!
//! The derust knows nothing about tokens or sessions: the handler decides that a failure is
//! recoverable and describes the recovery. The routes below (`/auth/refresh`, `/login`) belong to
//! this example application, not to derust. The vocabulary only exists in the `derust-stac` fork.
//! These builders are shared with the test that compares them with
//! `fixtures_reactive/error_envelope`.
use axum::http::StatusCode;
use derust::httpx::stac_error::StacErrorResponse;
use derust::httpx::stac_error::StacErrorResponseBuilder;
use derust::sduix::stacx::v1::prelude::*;

fn login_route() -> NavigateAction {
    NavigateAction::new()
        .route_name("/login")
        .navigation_style(NavigationStyle::PushReplacement)
}

/// Only the status and the version.
pub fn minimal() -> StacErrorResponseBuilder {
    StacErrorResponse::new(StatusCode::NOT_FOUND)
}

/// A screen rendered by the client in place of the failed one.
pub fn screen_only() -> StacErrorResponseBuilder {
    StacErrorResponse::new(StatusCode::NOT_FOUND).screen(Widget::from(
        column().children([Widget::from(text("Not found"))]),
    ))
}

/// An action the client runs after the error callbacks.
pub fn action_only() -> StacErrorResponseBuilder {
    StacErrorResponse::new(StatusCode::FORBIDDEN).action(Action::from(login_route()))
}

/// The session was refused and cannot be renewed: the server asks the client to log in again.
pub fn session_cycle() -> StacErrorResponseBuilder {
    // runs in the client; `response` is the response of the refresh request
    let refresh = NetworkRequest::new("/auth/refresh")
        .method(Method::Post)
        .on_success(Actions::Many(vec![
            Action::from(set_state(
                "global.token",
                StatePath::response().field("body").field("token"),
            )),
            Action::from(recovery_outcome(RecoveryResult::Success)),
        ]))
        .on_error(Action::from(recovery_outcome(RecoveryResult::Failure)));

    let recovery = Recovery::new()
        .key("session")
        .recover(Action::from(refresh))
        .retry(Retry::new().header(
            "Authorization",
            Binding::expr(Expr::text("Bearer ") + StatePath::global("token")),
        ))
        .on_failure(Action::from(login_route()))
        .on_unavailable(Action::from(set_state("global.offline", true)))
        .timeout_ms(10_000);

    StacErrorResponse::new(StatusCode::UNAUTHORIZED)
        .code("TOKEN_EXPIRED")
        .message("Session expired")
        .recovery(recovery)
}

/// Every field at once.
pub fn full() -> StacErrorResponseBuilder {
    session_cycle()
        .screen(Widget::from(
            column().children([Widget::from(text("Signing you in again"))]),
        ))
        .action(Action::from(set_state("global.lastError", "session")))
}
