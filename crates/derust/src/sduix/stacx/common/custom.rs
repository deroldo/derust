use super::node::NodeKind;
use super::validate::{Path, Validate, ValidationIssue};
use serde::ser::{Error as _, Serialize, SerializeMap, Serializer};
use serde_json::{Map, Value};
use std::fmt;
use std::sync::Arc;

/// A widget implemented by a custom parser registered in the Flutter app (not part of Stac).
///
/// The struct serializes its own fields (it must serialize as a JSON object); the `type`
/// discriminator is added from [`CustomWidget::TYPE_NAME`]. Any `T: CustomWidget` converts into
/// the `Widget` enum of every Stac major, so it fits wherever a standard widget does.
pub trait CustomWidget: Serialize + Validate + fmt::Debug + Send + Sync + 'static {
    /// Value of the `type` discriminator (the name the custom parser is registered with).
    const TYPE_NAME: &'static str;
}

/// An action implemented by a custom action parser registered in the Flutter app.
///
/// Same contract as [`CustomWidget`], with the `actionType` discriminator taken from
/// [`CustomAction::ACTION_TYPE`].
pub trait CustomAction: Serialize + Validate + fmt::Debug + Send + Sync + 'static {
    /// Value of the `actionType` discriminator.
    const ACTION_TYPE: &'static str;
}

/// Object safe view of a custom node.
trait ErasedNode: fmt::Debug + Send + Sync {
    fn to_json(&self) -> Result<Value, serde_json::Error>;
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>);
}

impl<T: Serialize + Validate + fmt::Debug + Send + Sync> ErasedNode for T {
    fn to_json(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        Validate::validate_at(self, path, issues);
    }
}

/// A custom widget or action stored in the `Custom` variant of `Widget` / `Action`.
#[derive(Clone)]
pub struct CustomNode {
    kind: NodeKind,
    type_name: &'static str,
    inner: Arc<dyn ErasedNode>,
}

impl CustomNode {
    /// Wraps a custom widget.
    pub fn widget<T: CustomWidget>(value: T) -> Self {
        Self {
            kind: NodeKind::Widget,
            type_name: T::TYPE_NAME,
            inner: Arc::new(value),
        }
    }

    /// Wraps a custom action.
    pub fn action<T: CustomAction>(value: T) -> Self {
        Self {
            kind: NodeKind::Action,
            type_name: T::ACTION_TYPE,
            inner: Arc::new(value),
        }
    }

    /// Whether the node is a widget or an action.
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Value of the discriminator (`type` / `actionType`).
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Fields of the node, without the discriminator.
    fn fields(&self) -> Result<Map<String, Value>, String> {
        let key = self.kind.discriminator_key();
        match self.inner.to_json() {
            Ok(Value::Object(map)) if map.contains_key(key) => Err(format!(
                "custom node `{}` has a `{key}` field, which clashes with the discriminator",
                self.type_name
            )),
            Ok(Value::Object(map)) => Ok(map),
            Ok(_) => Err(format!(
                "custom node `{}` must serialize as a JSON object",
                self.type_name
            )),
            Err(e) => Err(format!(
                "custom node `{}` failed to serialize: {e}",
                self.type_name
            )),
        }
    }
}

impl fmt::Debug for CustomNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomNode")
            .field("type_name", &self.type_name)
            .field("value", &self.inner)
            .finish()
    }
}

impl PartialEq for CustomNode {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.type_name == other.type_name
            && (Arc::ptr_eq(&self.inner, &other.inner)
                || matches!((self.inner.to_json(), other.inner.to_json()), (Ok(a), Ok(b)) if a == b))
    }
}

impl Serialize for CustomNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let fields = self.fields().map_err(S::Error::custom)?;
        let mut map = serializer.serialize_map(Some(fields.len() + 1))?;
        map.serialize_entry(self.kind.discriminator_key(), self.type_name)?;
        for (key, value) in &fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl Validate for CustomNode {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if let Err(message) = self.fields() {
            issues.push(ValidationIssue::new(path, message));
        }
        self.inner.validate_at(path, issues);
    }
}
