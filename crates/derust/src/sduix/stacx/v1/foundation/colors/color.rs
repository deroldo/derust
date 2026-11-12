use crate::sduix::stacx::common::{Dyn, Path, StateRef, Validate, ValidationIssue};
use serde::{Serialize, Serializer};

/// Stac color (`StacColor`, a `String` typedef in Dart).
///
/// Either a hex string such as `"#FF0000"` / `"#80FF0000"` (ARGB), a theme color name such as
/// `"primary"`, or a reference to the application state. Append `@<percent>` (see
/// [`Color::with_opacity`]) to override the opacity.
#[derive(Debug, Clone, PartialEq)]
pub struct Color(Dyn<String>);

impl Color {
    /// Creates a color from its Stac string representation.
    pub fn new(value: impl Into<Dyn<String>>) -> Self {
        Self(value.into())
    }

    /// Creates a color from a hex string (e.g. `"#FF0000"`).
    pub fn hex(value: impl Into<String>) -> Self {
        Self(Dyn::Literal(value.into()))
    }

    /// Same as Stac's `StacColor.withOpacity`: opacity is clamped to `0.0..=1.0` and written as
    /// an integer percentage suffix (`#FF0000@50`).
    pub fn with_opacity(self, opacity: f64) -> Self {
        match self.0 {
            Dyn::Literal(value) => {
                let base = value.split('@').next().unwrap_or_default();
                let percent = (opacity.clamp(0.0, 1.0) * 100.0).round() as i64;
                Self(Dyn::Literal(format!("{base}@{percent}")))
            }
            other => Self(other),
        }
    }
}

impl From<&str> for Color {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Color {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<StateRef> for Color {
    fn from(value: StateRef) -> Self {
        Self::new(value)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl Validate for Color {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.0.validate_at(path, issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_matches_stac_extension() {
        let color = Color::hex("#FF0000@10").with_opacity(0.5);
        assert_eq!(serde_json::to_value(color).unwrap(), "#FF0000@50");
        assert_eq!(
            serde_json::to_value(Color::hex("#00FF00").with_opacity(2.0)).unwrap(),
            "#00FF00@100"
        );
    }
}
