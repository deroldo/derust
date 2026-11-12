use super::validate::{Path, Validate, ValidationIssue};
use serde::ser::{Serialize, SerializeMap, Serializer};
use std::fmt;

/// Reference to a value stored in the Stac application state.
///
/// Serializes to the `{{key}}` template that the Stac client replaces at render time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateRef(String);

impl StateRef {
    /// Creates a reference to the state entry `key`.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// State key this reference points to.
    pub fn key(&self) -> &str {
        &self.0
    }

    /// The `{{key}}` template string.
    pub fn template(&self) -> String {
        format!("{{{{{}}}}}", self.0)
    }
}

impl fmt::Display for StateRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{{{}}}}}", self.0)
    }
}

impl Serialize for StateRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl Validate for StateRef {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        let key = self.0.trim();
        if key.is_empty() {
            issues.push(ValidationIssue::new(
                path,
                "state reference has an empty key",
            ));
        } else if key.contains("{{") || key.contains("}}") {
            issues.push(ValidationIssue::new(
                path,
                format!("state reference key `{key}` must not contain `{{{{` or `}}}}`"),
            ));
        }
    }
}

/// A value that is either a literal or a reference to the application state.
#[derive(Debug, Clone, PartialEq)]
pub enum Dyn<T> {
    /// Literal value, serialized as is.
    Literal(T),
    /// Reference to the application state, serialized as a `{{key}}` template.
    Ref(StateRef),
}

impl<T> Dyn<T> {
    /// Creates a literal value.
    pub fn literal(value: T) -> Self {
        Dyn::Literal(value)
    }

    /// Creates a state reference.
    pub fn state(key: impl Into<String>) -> Self {
        Dyn::Ref(StateRef::new(key))
    }
}

impl<T: Serialize> Serialize for Dyn<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Dyn::Literal(value) => value.serialize(serializer),
            Dyn::Ref(state) => state.serialize(serializer),
        }
    }
}

impl<T: Validate> Validate for Dyn<T> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            Dyn::Literal(value) => value.validate_at(path, issues),
            Dyn::Ref(state) => state.validate_at(path, issues),
        }
    }
}

impl From<&str> for Dyn<String> {
    fn from(value: &str) -> Self {
        Dyn::Literal(value.to_string())
    }
}

impl From<String> for Dyn<String> {
    fn from(value: String) -> Self {
        Dyn::Literal(value)
    }
}

impl From<StateRef> for Dyn<String> {
    fn from(value: StateRef) -> Self {
        Dyn::Ref(value)
    }
}

/// One `{"key": ..., "value": ...}` entry written to the application state
/// (used by `SetValue` widgets/actions).
#[derive(Debug, Clone, PartialEq)]
pub struct StateEntry {
    /// State key to write.
    pub key: String,
    /// Value to store.
    pub value: serde_json::Value,
}

impl StateEntry {
    /// Creates an entry.
    pub fn new(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl Serialize for StateEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("key", &self.key)?;
        map.serialize_entry("value", &self.value)?;
        map.end()
    }
}

impl Validate for StateEntry {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if self.key.trim().is_empty() {
            issues.push(ValidationIssue::new(&path.key("key"), "state key is empty"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_ref_serializes_as_template() {
        let value = serde_json::to_value(StateRef::new("user.name")).unwrap();
        assert_eq!(value, serde_json::json!("{{user.name}}"));
    }

    #[test]
    fn dyn_literal_and_ref() {
        let literal: Dyn<String> = "hello".into();
        let reference: Dyn<String> = StateRef::new("greeting").into();
        assert_eq!(serde_json::to_value(literal).unwrap(), "hello");
        assert_eq!(serde_json::to_value(reference).unwrap(), "{{greeting}}");
    }

    #[test]
    fn invalid_refs_are_reported() {
        assert_eq!(Dyn::<String>::state("").validate().len(), 1);
        assert_eq!(Dyn::<String>::state("a{{b").validate().len(), 1);
        assert!(Dyn::<String>::state("ok").validate().is_empty());
    }

    #[test]
    fn state_entry_serializes() {
        let entry = StateEntry::new("count", 3);
        assert_eq!(
            serde_json::to_value(entry).unwrap(),
            serde_json::json!({"key": "count", "value": 3})
        );
    }
}
