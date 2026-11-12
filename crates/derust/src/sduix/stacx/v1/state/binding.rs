use super::expr::Expr;
use super::path::StatePath;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::ser::{Serialize, SerializeMap, Serializer};

/// A binding: a JSON object with exactly one key, `$state` or `$expr`, whose value is text.
///
/// ```
/// use derust::sduix::stacx::v1::state::{Binding, Expr, StatePath};
/// use serde_json::json;
///
/// let state = Binding::from(StatePath::global("token"));
/// assert_eq!(serde_json::to_value(&state).unwrap(), json!({"$state": "global.token"}));
///
/// let expr = Binding::from(Expr::length(StatePath::key("tags")).gt(0));
/// assert_eq!(serde_json::to_value(&expr).unwrap(), json!({"$expr": "length(tags) > 0"}));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// `{"$state": "path"}`: a plain read of the state.
    State(StatePath),
    /// `{"$expr": "source"}`: an expression.
    Expr(Expr),
}

impl Binding {
    /// `{"$state": ..}` binding.
    pub fn state(path: StatePath) -> Self {
        Binding::State(path)
    }

    /// `{"$expr": ..}` binding.
    pub fn expr(expr: Expr) -> Self {
        Binding::Expr(expr)
    }
}

impl From<StatePath> for Binding {
    fn from(value: StatePath) -> Self {
        Binding::State(value)
    }
}

impl From<Expr> for Binding {
    fn from(value: Expr) -> Self {
        Binding::Expr(value)
    }
}

impl Serialize for Binding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Binding::State(path) => map.serialize_entry("$state", &path.to_string())?,
            Binding::Expr(expr) => map.serialize_entry("$expr", &expr.to_source())?,
        }
        map.end()
    }
}

impl Validate for Binding {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match self {
            Binding::State(state) => state.validate_at(&path.key("$state"), issues),
            Binding::Expr(expr) => expr.validate_at(&path.key("$expr"), issues),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_a_single_pair() {
        let state = Binding::from(StatePath::key("user").field("name"));
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            json!({"$state": "user.name"})
        );
        let expr = Binding::from(Expr::from(StatePath::key("a")) + 1);
        assert_eq!(
            serde_json::to_value(&expr).unwrap(),
            json!({"$expr": "a + 1"})
        );
    }

    #[test]
    fn validation_reaches_the_inner_value() {
        let bad = Binding::from(StatePath::key("1a"));
        assert_eq!(bad.validate()[0].path, "$.$state.root");
        let bad = Binding::from(Expr::num(f64::NAN));
        assert_eq!(bad.validate()[0].path, "$.$expr");
    }
}
