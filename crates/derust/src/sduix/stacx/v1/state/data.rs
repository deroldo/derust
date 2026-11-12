use super::binding::Binding;
use super::expr::Expr;
use super::path::StatePath;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::ser::{Serialize, Serializer};
use std::collections::BTreeMap;

/// Free-form JSON data in which any position may be a binding.
///
/// Replaces `serde_json::Value` wherever the client resolves bindings (request bodies, action
/// arguments, state values). `Serialize` is transparent.
#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    /// JSON `null`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Floating point number.
    Num(f64),
    /// Text.
    Text(String),
    /// List.
    List(Vec<Data>),
    /// Object.
    ///
    /// An object with exactly one key, `$state` or `$expr`, whose value is text is rejected by
    /// `Validate`: the contract has no escape, so the client would take it for a binding.
    Object(BTreeMap<String, Data>),
    /// Binding.
    Bind(Binding),
}

impl Data {
    /// Object built from `(key, value)` pairs.
    pub fn object<K, V>(entries: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<Data>,
    {
        Data::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }

    /// List built from items.
    pub fn list<V: Into<Data>>(items: impl IntoIterator<Item = V>) -> Self {
        Data::List(items.into_iter().map(Into::into).collect())
    }
}

impl Serialize for Data {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Data::Null => serializer.serialize_unit(),
            Data::Bool(v) => serializer.serialize_bool(*v),
            Data::Int(v) => serializer.serialize_i64(*v),
            Data::Num(v) => serializer.serialize_f64(*v),
            Data::Text(v) => serializer.serialize_str(v),
            Data::List(v) => v.serialize(serializer),
            Data::Object(v) => v.serialize(serializer),
            Data::Bind(v) => v.serialize(serializer),
        }
    }
}

impl Validate for Data {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            Data::Num(v) => v.validate_at(path, issues),
            Data::List(items) => items.validate_at(path, issues),
            Data::Object(map) => {
                if map.len() == 1 {
                    for key in ["$state", "$expr"] {
                        if matches!(map.get(key), Some(Data::Text(_))) {
                            issues.push(ValidationIssue::new(
                                path,
                                format!(
                                    "object with the single key `{key}` would be read as a binding; use `Data::Bind`"
                                ),
                            ));
                        }
                    }
                }
                map.validate_at(path, issues);
            }
            Data::Bind(binding) => binding.validate_at(path, issues),
            Data::Null | Data::Bool(_) | Data::Int(_) | Data::Text(_) => {}
        }
    }
}

impl From<serde_json::Value> for Data {
    fn from(value: serde_json::Value) -> Self {
        use serde_json::Value;
        match value {
            Value::Null => Data::Null,
            Value::Bool(v) => Data::Bool(v),
            Value::Number(n) => match n.as_i64() {
                Some(i) => Data::Int(i),
                None => Data::Num(n.as_f64().unwrap_or(f64::NAN)),
            },
            Value::String(v) => Data::Text(v),
            Value::Array(items) => Data::List(items.into_iter().map(Data::from).collect()),
            Value::Object(map) => {
                Data::Object(map.into_iter().map(|(k, v)| (k, Data::from(v))).collect())
            }
        }
    }
}

impl From<Binding> for Data {
    fn from(value: Binding) -> Self {
        Data::Bind(value)
    }
}

impl From<Expr> for Data {
    fn from(value: Expr) -> Self {
        Data::Bind(Binding::Expr(value))
    }
}

impl From<StatePath> for Data {
    fn from(value: StatePath) -> Self {
        Data::Bind(Binding::State(value))
    }
}

impl From<bool> for Data {
    fn from(value: bool) -> Self {
        Data::Bool(value)
    }
}

macro_rules! data_int {
    ($($ty:ty),*) => { $(impl From<$ty> for Data {
        fn from(value: $ty) -> Self { Data::Int(i64::from(value)) }
    })* };
}
data_int!(i8, i16, i32, i64, u8, u16, u32);

impl From<f64> for Data {
    fn from(value: f64) -> Self {
        Data::Num(value)
    }
}

impl From<f32> for Data {
    fn from(value: f32) -> Self {
        Data::Num(f64::from(value))
    }
}

impl From<&str> for Data {
    fn from(value: &str) -> Self {
        Data::Text(value.to_string())
    }
}

impl From<String> for Data {
    fn from(value: String) -> Self {
        Data::Text(value)
    }
}

impl<T: Into<Data>> From<Vec<T>> for Data {
    fn from(value: Vec<T>) -> Self {
        Data::List(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Data>> From<Option<T>> for Data {
    fn from(value: Option<T>) -> Self {
        value.map_or(Data::Null, Into::into)
    }
}

impl<T: Into<Data>> FromIterator<T> for Data {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Data::List(iter.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_transparently() {
        let data = Data::object([
            ("name", Data::from("x")),
            ("n", Data::from(3)),
            ("ratio", Data::from(0.5)),
            ("none", Data::Null),
            ("ref", StatePath::key("a").into()),
            ("list", Data::list([1, 2])),
        ]);
        assert_eq!(
            serde_json::to_value(&data).unwrap(),
            json!({"name": "x", "n": 3, "ratio": 0.5, "none": null,
                   "ref": {"$state": "a"}, "list": [1, 2]})
        );
    }

    #[test]
    fn json_value_roundtrip() {
        let value = json!({"a": [1, 2.5, "t", null, true]});
        let data = Data::from(value.clone());
        assert_eq!(serde_json::to_value(&data).unwrap(), value);
    }

    #[test]
    fn binding_shaped_object_is_rejected() {
        let bad = Data::object([("$state", "a")]);
        assert_eq!(bad.validate().len(), 1);
        let bad = Data::object([("$expr", "a + 1")]);
        assert_eq!(bad.validate().len(), 1);
        let two = Data::object([("$state", "a"), ("other", "b")]);
        assert!(two.validate().is_empty());
        let non_text = Data::object([("$state", 1)]);
        assert!(non_text.validate().is_empty());
        let nested = Data::list([Data::object([("$expr", "x")])]);
        assert_eq!(nested.validate()[0].path, "$[0]");
    }

    #[test]
    fn non_finite_number_is_reported() {
        assert_eq!(Data::Num(f64::INFINITY).validate().len(), 1);
    }
}
