//! Recovery instruction of the Stac error envelope: [`Recovery`] and [`Retry`].
//!
//! The server decides that a failure is recoverable and describes how: the client runs
//! `recover` (an action or a list), then repeats the failed request once, optionally with
//! overrides ([`Retry`]). `Retry` has no URL and no method: the repetition always targets the
//! original request. Values accept [`Data`] (literal JSON, `$state` or `$expr`); a `null`
//! override removes the key.
//!
//! Validation is typed only (`Validate::validate`): the nested actions live outside a screen, so
//! the contextual checks over serialized screens do not apply.

use super::data::Data;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use crate::sduix::stacx::v1::state::Actions;
use serde::ser::{Error as _, Serialize, SerializeMap, Serializer};
use std::collections::BTreeMap;

/// Longest accepted `key`, in characters.
pub const MAX_RECOVERY_KEY_LEN: usize = 128;
/// Largest accepted `timeoutMs`.
pub const MAX_RECOVERY_TIMEOUT_MS: u32 = 600_000;

/// How the repeated request is built: overrides over the original one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Retry {
    headers: BTreeMap<String, Data>,
    query_parameters: BTreeMap<String, Data>,
    body: BTreeMap<String, Data>,
    remove_headers: Vec<String>,
}

impl Retry {
    /// Repetition with no overrides.
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides (or sets) a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<Data>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Overrides (or sets) a query parameter.
    pub fn query(mut self, name: impl Into<String>, value: impl Into<Data>) -> Self {
        self.query_parameters.insert(name.into(), value.into());
        self
    }

    /// Overrides (or sets) a top-level body key.
    pub fn body(mut self, name: impl Into<String>, value: impl Into<Data>) -> Self {
        self.body.insert(name.into(), value.into());
        self
    }

    /// Removes a body key (`null` override).
    pub fn remove_body_key(mut self, name: impl Into<String>) -> Self {
        self.body.insert(name.into(), Data::Null);
        self
    }

    /// Removes a header from the repeated request.
    pub fn remove_header(mut self, name: impl Into<String>) -> Self {
        self.remove_headers.push(name.into());
        self
    }

    fn has_overrides(&self) -> bool {
        !(self.headers.is_empty() && self.query_parameters.is_empty() && self.body.is_empty())
    }
}

fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

impl Serialize for Retry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        if self.has_overrides() {
            map.serialize_entry("overrides", &Overrides(self))?;
        }
        if !self.remove_headers.is_empty() {
            map.serialize_entry("removeHeaders", &self.remove_headers)?;
        }
        map.end()
    }
}

struct Overrides<'a>(&'a Retry);

impl Serialize for Overrides<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        if !self.0.headers.is_empty() {
            map.serialize_entry("headers", &self.0.headers)?;
        }
        if !self.0.query_parameters.is_empty() {
            map.serialize_entry("queryParameters", &self.0.query_parameters)?;
        }
        if !self.0.body.is_empty() {
            map.serialize_entry("body", &self.0.body)?;
        }
        map.end()
    }
}

impl Validate for Retry {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        let overrides = path.key("overrides");
        let headers = overrides.key("headers");
        for (name, value) in &self.headers {
            if !is_token(name) {
                issues.push(ValidationIssue::new(
                    &headers,
                    "header name is not a valid HTTP token",
                ));
            }
            value.validate_at(&headers.key(name), issues);
        }
        let query = overrides.key("queryParameters");
        for (name, value) in &self.query_parameters {
            if name.is_empty() {
                issues.push(ValidationIssue::new(
                    &query,
                    "query parameter name is empty",
                ));
            }
            value.validate_at(&query.key(name), issues);
        }
        let body = overrides.key("body");
        for (name, value) in &self.body {
            if name.is_empty() {
                issues.push(ValidationIssue::new(&body, "body key is empty"));
            }
            value.validate_at(&body.key(name), issues);
        }
        let removed = path.key("removeHeaders");
        for (index, name) in self.remove_headers.iter().enumerate() {
            if !is_token(name) {
                issues.push(ValidationIssue::new(
                    &removed.index(index),
                    "header name is not a valid HTTP token",
                ));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
enum KeyMode {
    /// `key` omitted: the runtime coalesces by the hash of `recover`.
    #[default]
    Unset,
    /// `"key": null`: never coalesce.
    NoCoalesce,
    /// Explicit key.
    Key(String),
}

/// Recovery instruction carried by the error envelope (`recovery`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recovery {
    key: KeyMode,
    recover: Option<Actions>,
    retry: Option<Retry>,
    on_failure: Option<Actions>,
    on_unavailable: Option<Actions>,
    timeout_ms: Option<u32>,
}

impl Recovery {
    /// Empty instruction; `recover` is mandatory (checked by `Validate`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Coalescing key: concurrent failures with the same key share one recovery.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = KeyMode::Key(key.into());
        self
    }

    /// Never coalesce (serializes `"key": null`).
    pub fn no_coalesce(mut self) -> Self {
        self.key = KeyMode::NoCoalesce;
        self
    }

    /// Action (or list) that performs the recovery.
    pub fn recover(mut self, actions: impl Into<Actions>) -> Self {
        self.recover = Some(actions.into());
        self
    }

    /// How the failed request is repeated after a successful recovery.
    pub fn retry(mut self, retry: Retry) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Runs when the recovery fails.
    pub fn on_failure(mut self, actions: impl Into<Actions>) -> Self {
        self.on_failure = Some(actions.into());
        self
    }

    /// Runs when the recovery cannot be attempted (service unavailable).
    pub fn on_unavailable(mut self, actions: impl Into<Actions>) -> Self {
        self.on_unavailable = Some(actions.into());
        self
    }

    /// Recovery timeout in milliseconds (1 to 600000).
    pub fn timeout_ms(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

impl Serialize for Recovery {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let recover = self
            .recover
            .as_ref()
            .ok_or_else(|| S::Error::custom("recovery without `recover`"))?;
        let mut map = serializer.serialize_map(None)?;
        match &self.key {
            KeyMode::Unset => {}
            KeyMode::NoCoalesce => map.serialize_entry("key", &None::<String>)?,
            KeyMode::Key(key) => map.serialize_entry("key", key)?,
        }
        map.serialize_entry("recover", recover)?;
        if let Some(retry) = &self.retry {
            map.serialize_entry("retry", retry)?;
        }
        if let Some(value) = &self.on_failure {
            map.serialize_entry("onFailure", value)?;
        }
        if let Some(value) = &self.on_unavailable {
            map.serialize_entry("onUnavailable", value)?;
        }
        if let Some(value) = self.timeout_ms {
            map.serialize_entry("timeoutMs", &value)?;
        }
        map.end()
    }
}

fn actions_empty(actions: &Actions) -> bool {
    matches!(actions, Actions::Many(list) if list.is_empty())
}

impl Validate for Recovery {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match &self.recover {
            None => issues.push(ValidationIssue::new(
                &path.key("recover"),
                "`recover` is required",
            )),
            Some(actions) if actions_empty(actions) => issues.push(ValidationIssue::new(
                &path.key("recover"),
                "`recover` must not be an empty list",
            )),
            Some(actions) => actions.validate_at(&path.key("recover"), issues),
        }
        if let KeyMode::Key(key) = &self.key {
            let at = path.key("key");
            if key.trim().is_empty() {
                issues.push(ValidationIssue::new(&at, "key is empty"));
            }
            if key.chars().count() > MAX_RECOVERY_KEY_LEN {
                issues.push(ValidationIssue::new(
                    &at,
                    format!("key is longer than {MAX_RECOVERY_KEY_LEN} characters"),
                ));
            }
            if key.chars().any(char::is_control) {
                issues.push(ValidationIssue::new(&at, "key has control characters"));
            }
        }
        if let Some(retry) = &self.retry {
            retry.validate_at(&path.key("retry"), issues);
        }
        for (name, actions) in [
            ("onFailure", &self.on_failure),
            ("onUnavailable", &self.on_unavailable),
        ] {
            if let Some(actions) = actions {
                actions.validate_at(&path.key(name), issues);
            }
        }
        if let Some(timeout) = self.timeout_ms {
            if !(1..=MAX_RECOVERY_TIMEOUT_MS).contains(&timeout) {
                issues.push(ValidationIssue::new(
                    &path.key("timeoutMs"),
                    format!("timeoutMs must be between 1 and {MAX_RECOVERY_TIMEOUT_MS}"),
                ));
            }
        }
    }
}
