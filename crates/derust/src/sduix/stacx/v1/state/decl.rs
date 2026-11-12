use super::data::Data;
use super::ident::Ident;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use crate::sduix::stacx::v1::actions::Action;
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::collections::BTreeMap;
use std::fmt;

/// Key a widget writes its value to (`bind`).
///
/// Serializes as `key` (screen state) or `global.key` (global state).
#[derive(Debug, Clone, PartialEq)]
pub enum BindKey {
    /// Screen state key.
    Screen(Ident),
    /// Global state key.
    Global(Ident),
}

impl BindKey {
    /// Screen state key.
    pub fn screen(name: impl Into<Ident>) -> Self {
        BindKey::Screen(name.into())
    }

    /// Global state key.
    pub fn global(name: impl Into<Ident>) -> Self {
        BindKey::Global(name.into())
    }
}

impl fmt::Display for BindKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindKey::Screen(name) => write!(f, "{name}"),
            BindKey::Global(name) => write!(f, "global.{name}"),
        }
    }
}

impl Serialize for BindKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl Validate for BindKey {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        let (BindKey::Screen(name) | BindKey::Global(name)) = self;
        if name.as_str().trim().is_empty() {
            issues.push(ValidationIssue::new(path, "bind key is empty"));
        } else {
            name.validate_at(path, issues);
        }
    }
}

/// Screen-scoped state declaration (`state`): key -> initial [`Data`].
///
/// Screen keys have no `persist`/`secure` (those only exist for global keys).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateBlock(pub BTreeMap<String, Data>);

impl StateBlock {
    /// Empty block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `key` with its initial value.
    pub fn with(mut self, key: impl Into<String>, initial: impl Into<Data>) -> Self {
        self.0.insert(key.into(), initial.into());
        self
    }
}

impl Serialize for StateBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl Validate for StateBlock {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.0.validate_at(path, issues);
    }
}

/// One entry of the global state declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum GlobalEntry {
    /// Plain initial value.
    Value(Data),
    /// Declared entry: `{"initial": .., "persist": .., "secure": ..}`.
    Declared {
        /// Initial value.
        initial: Data,
        /// Whether the value survives restarts.
        persist: Option<bool>,
        /// Whether the value is kept in secure storage.
        secure: Option<bool>,
    },
}

impl GlobalEntry {
    /// Plain initial value.
    pub fn value(initial: impl Into<Data>) -> Self {
        GlobalEntry::Value(initial.into())
    }

    /// Declared entry with only the initial value (add `persist`/`secure` with the setters).
    pub fn declared(initial: impl Into<Data>) -> Self {
        GlobalEntry::Declared {
            initial: initial.into(),
            persist: None,
            secure: None,
        }
    }

    fn into_declared(self) -> (Data, Option<bool>, Option<bool>) {
        match self {
            GlobalEntry::Value(initial) => (initial, None, None),
            GlobalEntry::Declared {
                initial,
                persist,
                secure,
            } => (initial, persist, secure),
        }
    }

    /// Sets `persist` (turns a plain value into a declared entry).
    pub fn persist(self, persist: bool) -> Self {
        let (initial, _, secure) = self.into_declared();
        GlobalEntry::Declared {
            initial,
            persist: Some(persist),
            secure,
        }
    }

    /// Sets `secure` (turns a plain value into a declared entry).
    pub fn secure(self, secure: bool) -> Self {
        let (initial, persist, _) = self.into_declared();
        GlobalEntry::Declared {
            initial,
            persist,
            secure: Some(secure),
        }
    }
}

impl Serialize for GlobalEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            GlobalEntry::Value(data) => data.serialize(serializer),
            GlobalEntry::Declared {
                initial,
                persist,
                secure,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("initial", initial)?;
                if let Some(persist) = persist {
                    map.serialize_entry("persist", persist)?;
                }
                if let Some(secure) = secure {
                    map.serialize_entry("secure", secure)?;
                }
                map.end()
            }
        }
    }
}

impl Validate for GlobalEntry {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            GlobalEntry::Value(data) => data.validate_at(path, issues),
            GlobalEntry::Declared {
                initial,
                persist,
                secure,
            } => {
                initial.validate_at(&path.key("initial"), issues);
                if *secure == Some(true) && *persist == Some(false) {
                    issues.push(ValidationIssue::new(
                        path,
                        "a `secure` entry cannot have `persist: false`",
                    ));
                }
            }
        }
    }
}

/// Global state declaration (`globalState`): key -> [`GlobalEntry`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlobalStateBlock(pub BTreeMap<String, GlobalEntry>);

impl GlobalStateBlock {
    /// Empty block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `key`.
    pub fn with(mut self, key: impl Into<String>, entry: GlobalEntry) -> Self {
        self.0.insert(key.into(), entry);
        self
    }
}

impl Serialize for GlobalStateBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl Validate for GlobalStateBlock {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.0.validate_at(path, issues);
    }
}

/// Actions run when a widget initializes (`onInit`): one action or a list.
///
/// A list is executed by the runtime as a synchronous `multiAction`.
#[derive(Debug, Clone, PartialEq)]
pub enum OnInit {
    /// A single action.
    One(Action),
    /// Several actions, in order.
    Many(Vec<Action>),
}

impl From<Action> for OnInit {
    fn from(value: Action) -> Self {
        OnInit::One(value)
    }
}

impl From<Vec<Action>> for OnInit {
    fn from(value: Vec<Action>) -> Self {
        OnInit::Many(value)
    }
}

impl Serialize for OnInit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OnInit::One(action) => action.serialize(serializer),
            OnInit::Many(actions) => actions.serialize(serializer),
        }
    }
}

impl Validate for OnInit {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            OnInit::One(action) => action.validate_at(path, issues),
            OnInit::Many(actions) => actions.validate_at(path, issues),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bind_key_forms() {
        assert_eq!(
            serde_json::to_value(BindKey::screen("name")).unwrap(),
            json!("name")
        );
        assert_eq!(
            serde_json::to_value(BindKey::global("token")).unwrap(),
            json!("global.token")
        );
        assert_eq!(BindKey::screen("").validate().len(), 1);
        assert!(BindKey::global("ok").validate().is_empty());
    }

    #[test]
    fn state_blocks_serialize() {
        let state = StateBlock::new().with("count", 0).with("name", "x");
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            json!({"count": 0, "name": "x"})
        );
        let global = GlobalStateBlock::new()
            .with("theme", GlobalEntry::value("dark"))
            .with(
                "token",
                GlobalEntry::declared(Data::Null).persist(true).secure(true),
            )
            .with("flag", GlobalEntry::declared(false));
        assert_eq!(
            serde_json::to_value(&global).unwrap(),
            json!({
                "theme": "dark",
                "token": {"initial": null, "persist": true, "secure": true},
                "flag": {"initial": false}
            })
        );
    }

    #[test]
    fn secure_with_persist_false_is_an_error() {
        let bad = GlobalEntry::declared(Data::Null)
            .secure(true)
            .persist(false);
        assert_eq!(bad.validate().len(), 1);
        let ok = GlobalEntry::declared(Data::Null).secure(true).persist(true);
        assert!(ok.validate().is_empty());
        let block = GlobalStateBlock::new().with("k", bad);
        assert_eq!(block.validate()[0].path, "$.k");
    }

    #[test]
    fn on_init_one_and_many() {
        let action = Action::from(crate::sduix::stacx::v1::actions::DelayAction::new());
        assert_eq!(
            serde_json::to_value(OnInit::One(action.clone())).unwrap()["actionType"],
            json!("delay")
        );
        let many = serde_json::to_value(OnInit::Many(vec![action.clone(), action])).unwrap();
        assert_eq!(many.as_array().map(Vec::len), Some(2));
    }
}
