//! Actions of the fork that read and write the reactive state (`setState`, `incrementState`,
//! `decrementState`, `toggleState`, `resetState`, `appendState`, `removeFromState`,
//! `toggleItem`, `cancelRequest`, `none`).
//!
//! Every action accepts `condition` (a `bool`, a state path or an expression). Keys accept the
//! `global.` prefix (`"global.theme"`). Contextual rules (undeclared keys, reserved names) are
//! checked over the serialized screen, not here.

use super::data::Data;
use super::decl::BindKey;
use super::reactive::{IntoReactive, Plain, Reactive};
use crate::sduix::stacx::common::{NodeKind, Path, StacNode, Validate, ValidationIssue};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::BTreeMap;

/// Common parts of every state action: the `condition` setter, `StacNode` and `Plain`.
macro_rules! action_common {
    ($ty:ident, $json:literal) => {
        impl $ty {
            /// Value of the `actionType` discriminator.
            pub const TYPE: &'static str = $json;

            /// Run the action only when this is true (state path or expression).
            pub fn condition(mut self, value: impl IntoReactive<bool>) -> Self {
                self.condition = Some(value.into_reactive());
                self
            }
        }

        impl StacNode for $ty {
            const KIND: NodeKind = NodeKind::Action;
            const TYPE_NAME: &'static str = $json;
        }

        impl Plain for $ty {}
    };
}

fn start<S: Serializer>(
    serializer: S,
    action_type: &str,
    condition: &Option<Reactive<bool>>,
) -> Result<S::SerializeMap, S::Error> {
    let mut map = serializer.serialize_map(None)?;
    map.serialize_entry("actionType", action_type)?;
    if let Some(value) = condition {
        map.serialize_entry("condition", value)?;
    }
    Ok(map)
}

fn validate_key(key: &BindKey, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
    key.validate_at(&path.key("key"), issues);
}

/// `setState`: writes several keys in one batch.
#[derive(Debug, Clone, PartialEq)]
pub struct SetState {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// Key (accepts `global.`) to value (or binding).
    pub values: BTreeMap<String, Data>,
}

impl SetState {
    /// Writes `value` to `key`; add more keys with [`SetState::and`].
    pub fn new(key: impl Into<String>, value: impl Into<Data>) -> Self {
        Self {
            condition: None,
            values: BTreeMap::from([(key.into(), value.into())]),
        }
    }

    /// Also writes `value` to `key` (same batch).
    pub fn and(mut self, key: impl Into<String>, value: impl Into<Data>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

action_common!(SetState, "setState");

impl Serialize for SetState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("values", &self.values)?;
        map.end()
    }
}

impl Validate for SetState {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        let values = path.key("values");
        for (key, value) in &self.values {
            if key.trim().is_empty() || key == "global." {
                issues.push(ValidationIssue::new(&values, "state key is empty"));
            }
            value.validate_at(&values.key(key), issues);
        }
    }
}

macro_rules! step_action {
    ($(#[$doc:meta])* $ty:ident, $json:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $ty {
            /// Run the action only when this is true.
            pub condition: Option<Reactive<bool>>,
            /// A key holding a number (accepts `global.`).
            pub key: BindKey,
            /// Step (default `1` in the client).
            pub by: Option<Reactive<f64>>,
            /// Inclusive lower bound, applied to the result.
            pub min: Option<Reactive<f64>>,
            /// Inclusive upper bound, applied to the result.
            pub max: Option<Reactive<f64>>,
        }

        impl $ty {
            /// Action over `key`.
            pub fn new(key: impl Into<BindKey>) -> Self {
                Self { condition: None, key: key.into(), by: None, min: None, max: None }
            }

            /// Step.
            pub fn by(mut self, value: impl IntoReactive<f64>) -> Self {
                self.by = Some(value.into_reactive());
                self
            }

            /// Inclusive lower bound.
            pub fn min(mut self, value: impl IntoReactive<f64>) -> Self {
                self.min = Some(value.into_reactive());
                self
            }

            /// Inclusive upper bound.
            pub fn max(mut self, value: impl IntoReactive<f64>) -> Self {
                self.max = Some(value.into_reactive());
                self
            }
        }

        action_common!($ty, $json);

        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = start(serializer, Self::TYPE, &self.condition)?;
                map.serialize_entry("key", &self.key)?;
                if let Some(value) = &self.by {
                    map.serialize_entry("by", value)?;
                }
                if let Some(value) = &self.min {
                    map.serialize_entry("min", value)?;
                }
                if let Some(value) = &self.max {
                    map.serialize_entry("max", value)?;
                }
                map.end()
            }
        }

        impl Validate for $ty {
            fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
                self.condition.validate_at(&path.key("condition"), issues);
                validate_key(&self.key, path, issues);
                self.by.validate_at(&path.key("by"), issues);
                self.min.validate_at(&path.key("min"), issues);
                self.max.validate_at(&path.key("max"), issues);
            }
        }
    };
}

step_action!(
    /// `incrementState`: `new = current + by`, then clamped to `min` / `max`.
    IncrementState,
    "incrementState"
);
step_action!(
    /// `decrementState`: `new = current - by`, then clamped to `min` / `max`.
    DecrementState,
    "decrementState"
);

/// `toggleState`: inverts a `bool` key.
#[derive(Debug, Clone, PartialEq)]
pub struct ToggleState {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// A key holding a `bool` (accepts `global.`).
    pub key: BindKey,
}

impl ToggleState {
    /// Action over `key`.
    pub fn new(key: impl Into<BindKey>) -> Self {
        Self {
            condition: None,
            key: key.into(),
        }
    }
}

action_common!(ToggleState, "toggleState");

impl Serialize for ToggleState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("key", &self.key)?;
        map.end()
    }
}

impl Validate for ToggleState {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        validate_key(&self.key, path, issues);
    }
}

/// `resetState`: restores keys to the value they had at declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ResetState {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// Keys to restore; without it, every screen key (global keys only when listed).
    pub keys: Option<Vec<BindKey>>,
}

impl ResetState {
    /// Resets every screen key.
    pub fn new() -> Self {
        Self {
            condition: None,
            keys: None,
        }
    }

    /// Resets only these keys (`"global.theme"` for a global key).
    pub fn keys(mut self, keys: impl IntoIterator<Item = impl Into<BindKey>>) -> Self {
        self.keys = Some(keys.into_iter().map(Into::into).collect());
        self
    }
}

impl Default for ResetState {
    fn default() -> Self {
        Self::new()
    }
}

action_common!(ResetState, "resetState");

impl Serialize for ResetState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        if let Some(keys) = &self.keys {
            map.serialize_entry("keys", keys)?;
        }
        map.end()
    }
}

impl Validate for ResetState {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        self.keys.validate_at(&path.key("keys"), issues);
    }
}

/// `appendState`: adds an item at the end of a list key.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendState {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// A key holding a list (accepts `global.`).
    pub key: BindKey,
    /// Item to add (`null` when absent).
    pub value: Option<Data>,
}

impl AppendState {
    /// Action over `key`.
    pub fn new(key: impl Into<BindKey>) -> Self {
        Self {
            condition: None,
            key: key.into(),
            value: None,
        }
    }

    /// Item to add (a literal or a binding).
    pub fn value(mut self, value: impl Into<Data>) -> Self {
        self.value = Some(value.into());
        self
    }
}

action_common!(AppendState, "appendState");

impl Serialize for AppendState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("key", &self.key)?;
        if let Some(value) = &self.value {
            map.serialize_entry("value", value)?;
        }
        map.end()
    }
}

impl Validate for AppendState {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        validate_key(&self.key, path, issues);
        self.value.validate_at(&path.key("value"), issues);
    }
}

/// `removeFromState`: removes an item of a list key by position or by value.
///
/// Built only with [`RemoveFromState::at_index`] or [`RemoveFromState::by_value`], so an action
/// without a target cannot be represented.
///
/// ```compile_fail
/// use derust::sduix::stacx::v1::prelude::*;
/// // there is no way to build the action without `index` or `value`
/// let _ = RemoveFromState::new("todos");
/// ```
///
/// ```compile_fail
/// use derust::sduix::stacx::v1::prelude::*;
/// use derust::sduix::stacx::v1::state::RemoveFromState;
/// // fields are private
/// let _ = RemoveFromState { condition: None, key: "todos".into(), index: None, value: None };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromState {
    condition: Option<Reactive<bool>>,
    key: BindKey,
    index: Option<Reactive<i64>>,
    value: Option<Data>,
}

impl RemoveFromState {
    /// Removes the item at `index` (from 0); a binding is accepted (e.g. the template `index`).
    pub fn at_index(key: impl Into<BindKey>, index: impl IntoReactive<i64>) -> Self {
        Self {
            condition: None,
            key: key.into(),
            index: Some(index.into_reactive()),
            value: None,
        }
    }

    /// Removes the first item equal to `value` (deep equality).
    pub fn by_value(key: impl Into<BindKey>, value: impl Into<Data>) -> Self {
        Self {
            condition: None,
            key: key.into(),
            index: None,
            value: Some(value.into()),
        }
    }

    /// Also sends `value` (the client gives `index` priority when both are present).
    pub fn also_value(mut self, value: impl Into<Data>) -> Self {
        self.value = Some(value.into());
        self
    }
}

action_common!(RemoveFromState, "removeFromState");

impl Serialize for RemoveFromState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("key", &self.key)?;
        if let Some(value) = &self.index {
            map.serialize_entry("index", value)?;
        }
        if let Some(value) = &self.value {
            map.serialize_entry("value", value)?;
        }
        map.end()
    }
}

impl Validate for RemoveFromState {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        validate_key(&self.key, path, issues);
        self.index.validate_at(&path.key("index"), issues);
        self.value.validate_at(&path.key("value"), issues);
    }
}

/// `toggleItem`: removes an equal item from a list key, or appends `value`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToggleItem {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// A key holding a list (accepts `global.`).
    pub key: BindKey,
    /// Item to add or remove (`null` when absent).
    pub value: Option<Data>,
    /// Field used as identity when items are objects.
    pub by: Option<String>,
}

impl ToggleItem {
    /// Action over `key`.
    pub fn new(key: impl Into<BindKey>) -> Self {
        Self {
            condition: None,
            key: key.into(),
            value: None,
            by: None,
        }
    }

    /// Item to add or remove (a literal or a binding, e.g. `item.id`).
    pub fn value(mut self, value: impl Into<Data>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Field used as identity when items are objects.
    pub fn by(mut self, field: impl Into<String>) -> Self {
        self.by = Some(field.into());
        self
    }
}

action_common!(ToggleItem, "toggleItem");

impl Serialize for ToggleItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("key", &self.key)?;
        if let Some(value) = &self.value {
            map.serialize_entry("value", value)?;
        }
        if let Some(value) = &self.by {
            map.serialize_entry("by", value)?;
        }
        map.end()
    }
}

impl Validate for ToggleItem {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        validate_key(&self.key, path, issues);
        self.value.validate_at(&path.key("value"), issues);
        if self.by.as_deref().is_some_and(|by| by.trim().is_empty()) {
            issues.push(ValidationIssue::new(&path.key("by"), "field name is empty"));
        }
    }
}

/// `cancelRequest`: cancels the in-flight request named `request_key` on the current screen.
#[derive(Debug, Clone, PartialEq)]
pub struct CancelRequest {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
    /// Key given to the request (`requestKey`).
    pub request_key: String,
}

impl CancelRequest {
    /// Action cancelling `request_key`.
    pub fn new(request_key: impl Into<String>) -> Self {
        Self {
            condition: None,
            request_key: request_key.into(),
        }
    }
}

action_common!(CancelRequest, "cancelRequest");

impl Serialize for CancelRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = start(serializer, Self::TYPE, &self.condition)?;
        map.serialize_entry("requestKey", &self.request_key)?;
        map.end()
    }
}

impl Validate for CancelRequest {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
        if self.request_key.trim().is_empty() {
            issues.push(ValidationIssue::new(
                &path.key("requestKey"),
                "request key is empty",
            ));
        }
    }
}

/// `none`: does nothing (e.g. an `onCancel` that ignores the cancellation).
#[derive(Debug, Clone, PartialEq)]
pub struct NoneAction {
    /// Run the action only when this is true.
    pub condition: Option<Reactive<bool>>,
}

impl NoneAction {
    /// Action that does nothing.
    pub fn new() -> Self {
        Self { condition: None }
    }
}

impl Default for NoneAction {
    fn default() -> Self {
        Self::new()
    }
}

action_common!(NoneAction, "none");

impl Serialize for NoneAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        start(serializer, Self::TYPE, &self.condition)?.end()
    }
}

impl Validate for NoneAction {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.condition.validate_at(&path.key("condition"), issues);
    }
}

/// Shortcut for [`SetState::new`].
pub fn set_state(key: impl Into<String>, value: impl Into<Data>) -> SetState {
    SetState::new(key, value)
}

/// Shortcut for [`IncrementState::new`].
pub fn increment_state(key: impl Into<BindKey>) -> IncrementState {
    IncrementState::new(key)
}

/// Shortcut for [`DecrementState::new`].
pub fn decrement_state(key: impl Into<BindKey>) -> DecrementState {
    DecrementState::new(key)
}

/// Shortcut for [`ToggleState::new`].
pub fn toggle_state(key: impl Into<BindKey>) -> ToggleState {
    ToggleState::new(key)
}

/// Shortcut for [`ResetState::new`].
pub fn reset_state() -> ResetState {
    ResetState::new()
}

/// Shortcut for [`AppendState::new`].
pub fn append_state(key: impl Into<BindKey>) -> AppendState {
    AppendState::new(key)
}

/// Shortcut for [`RemoveFromState::at_index`].
pub fn remove_from_state_at(
    key: impl Into<BindKey>,
    index: impl IntoReactive<i64>,
) -> RemoveFromState {
    RemoveFromState::at_index(key, index)
}

/// Shortcut for [`RemoveFromState::by_value`].
pub fn remove_from_state_value(key: impl Into<BindKey>, value: impl Into<Data>) -> RemoveFromState {
    RemoveFromState::by_value(key, value)
}

/// Shortcut for [`ToggleItem::new`].
pub fn toggle_item(key: impl Into<BindKey>) -> ToggleItem {
    ToggleItem::new(key)
}

/// Shortcut for [`CancelRequest::new`].
pub fn cancel_request(request_key: impl Into<String>) -> CancelRequest {
    CancelRequest::new(request_key)
}

/// Shortcut for [`NoneAction::new`].
pub fn none_action() -> NoneAction {
    NoneAction::new()
}
