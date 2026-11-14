//! Format rules of `navigate.routeName` (see `docs/actions/navigate.mdx` of the fork).
//!
//! The runtime does not impose a format: the value goes to `RouteSettings(name:)` and, on the web,
//! through `Uri.parse`. When the screen comes from `request` / `widgetJson` / `assetPath` the name
//! is the page route (the URL on the web, visible in the address bar and in the history), so the
//! typed model requires a relative `/screens/...` path there. Messages never echo the value.

/// Prefix required when `routeName` names the page route of a loaded screen.
pub const SCREEN_ROUTE_PREFIX: &str = "/screens/";

/// Returns why the literal `value` is not a valid `routeName`, or `None`.
///
/// `names_loaded_screen` is true when the action has `request`, `widgetJson` or `assetPath` and its
/// style is push, pushReplacement, pushAndRemoveAll or absent: the strict rules apply. Otherwise only
/// the basic rules do (not empty, no whitespace or control characters), so a Stac screen name
/// (`detail_screen`) or a Flutter route (`/settings`) stays valid. Legacy `{{key}}` templates are
/// resolved at runtime and are not checked.
pub(crate) fn route_name_problem(value: &str, names_loaded_screen: bool) -> Option<&'static str> {
    if value.contains("{{") && value.contains("}}") {
        return None;
    }
    if value.is_empty() {
        return Some("routeName must not be empty");
    }
    if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Some("routeName must not contain spaces or control characters");
    }
    if !names_loaded_screen {
        return None;
    }
    if value.contains('\\') {
        return Some("routeName must not contain a backslash");
    }
    if value.starts_with("//") {
        return Some("routeName must be a relative path, not start with `//`");
    }
    let before_slash = value.split('/').next().unwrap_or("");
    if value.contains("://") || before_slash.contains(':') {
        return Some("routeName must be a relative path, without scheme or host");
    }
    if !value.starts_with(SCREEN_ROUTE_PREFIX) {
        return Some(
            "routeName must start with `/screens/` when the action loads a screen \
             (request, widgetJson or assetPath)",
        );
    }
    None
}
