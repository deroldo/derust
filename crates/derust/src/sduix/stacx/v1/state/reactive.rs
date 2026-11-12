use super::binding::Binding;
use super::data::Data;
use super::expr::Expr;
use super::path::StatePath;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::ser::{Serialize, Serializer};

/// A property value that is either a literal or a binding to the reactive state.
///
/// Serializes transparently: a literal as itself, a binding as `{"$state": ..}` / `{"$expr": ..}`.
#[derive(Debug, Clone, PartialEq)]
pub enum Reactive<T> {
    /// Literal value.
    Value(T),
    /// Binding resolved by the client at render time.
    Bind(Binding),
}

impl<T> Reactive<T> {
    /// Literal value.
    pub fn value(value: T) -> Self {
        Reactive::Value(value)
    }

    /// Binding (`StatePath`, `Expr` or `Binding`).
    pub fn bind(binding: impl Into<Binding>) -> Self {
        Reactive::Bind(binding.into())
    }
}

impl<T: Serialize> Serialize for Reactive<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Reactive::Value(value) => value.serialize(serializer),
            Reactive::Bind(binding) => binding.serialize(serializer),
        }
    }
}

impl<T: Validate> Validate for Reactive<T> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            Reactive::Value(value) => value.validate_at(path, issues),
            Reactive::Bind(binding) => binding.validate_at(path, issues),
        }
    }
}

impl<T> From<Binding> for Reactive<T> {
    fn from(value: Binding) -> Self {
        Reactive::Bind(value)
    }
}

/// Marker for plain (non-binding) values that can be used directly as `Reactive<T>`.
///
/// Public only so generated code and applications can name it; it is sealed by convention:
/// do not implement it outside of this crate (it keeps the `IntoReactive` impls disjoint).
pub trait Plain {}

macro_rules! plain {
    ($($ty:ty),* $(,)?) => { $(impl Plain for $ty {})* };
}

plain!(bool, i64, f64, String, serde_json::Value, Data);
impl<T: Plain> Plain for Vec<T> {}

/// Anything accepted by a setter of a property of type `T`: a literal, a [`Binding`],
/// an [`Expr`], a [`StatePath`] or an existing [`Reactive`].
pub trait IntoReactive<T> {
    /// Converts into the stored form.
    fn into_reactive(self) -> Reactive<T>;
}

impl<T: Plain> IntoReactive<T> for T {
    fn into_reactive(self) -> Reactive<T> {
        Reactive::Value(self)
    }
}

impl<T> IntoReactive<T> for Reactive<T> {
    fn into_reactive(self) -> Reactive<T> {
        self
    }
}

impl<T> IntoReactive<T> for Binding {
    fn into_reactive(self) -> Reactive<T> {
        Reactive::Bind(self)
    }
}

impl<T> IntoReactive<T> for Expr {
    fn into_reactive(self) -> Reactive<T> {
        Reactive::Bind(Binding::Expr(self))
    }
}

impl<T> IntoReactive<T> for StatePath {
    fn into_reactive(self) -> Reactive<T> {
        Reactive::Bind(Binding::State(self))
    }
}

/// Compatibility conversions: `$from` is accepted where `$to` is stored.
macro_rules! compat {
    ($to:ty: $($from:ty),* $(,)?) => {
        $(impl IntoReactive<$to> for $from {
            fn into_reactive(self) -> Reactive<$to> {
                Reactive::Value(<$to>::from(self))
            }
        })*
    };
}

compat!(f64: f32, i8, i16, i32, u8, u16, u32);
compat!(i64: i8, i16, i32, u8, u16, u32);
compat!(String: &str, &String, char);

impl IntoReactive<Vec<String>> for Vec<&str> {
    fn into_reactive(self) -> Reactive<Vec<String>> {
        Reactive::Value(self.into_iter().map(String::from).collect())
    }
}

impl<const N: usize> IntoReactive<Vec<String>> for [&str; N] {
    fn into_reactive(self) -> Reactive<Vec<String>> {
        Reactive::Value(self.into_iter().map(String::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_transparently() {
        let lit: Reactive<f64> = 8.into_reactive();
        assert_eq!(serde_json::to_value(&lit).unwrap(), json!(8.0));
        let state: Reactive<bool> = StatePath::key("a").field("b").index(0).into_reactive();
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            json!({"$state": "a.b[0]"})
        );
        let expr: Reactive<bool> = Expr::from(StatePath::key("a")).gt(1).into_reactive();
        assert_eq!(
            serde_json::to_value(&expr).unwrap(),
            json!({"$expr": "a > 1"})
        );
    }

    #[test]
    fn validation_reaches_the_binding() {
        let bad: Reactive<f64> = Expr::num(f64::NAN).into_reactive();
        assert_eq!(bad.validate().len(), 1);
        assert!(Reactive::<f64>::value(1.0).validate().is_empty());
    }
}
