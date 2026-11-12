use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::{Serialize, Serializer};
use std::fmt;

/// Identifier of the expression language: `[A-Za-z_][A-Za-z0-9_]*`, never `true`, `false`
/// or `null` (those are literals for the Stac parser, even as a field name).
///
/// Construction never fails; [`Validate`] reports invalid identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ident(String);

impl Ident {
    /// Wraps `name` without checking it (see [`Ident::is_valid`] and [`Validate`]).
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether `name` matches `[A-Za-z_][A-Za-z0-9_]*`.
    pub fn is_lexically_valid(name: &str) -> bool {
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Whether `name` is a valid identifier (lexically valid and not a literal keyword).
    pub fn is_valid(name: &str) -> bool {
        Self::is_lexically_valid(name) && !matches!(name, "true" | "false" | "null")
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Ident {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Ident {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Serialize for Ident {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl Validate for Ident {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        // The message never echoes the text: only the rule that was broken.
        if !Self::is_lexically_valid(&self.0) {
            issues.push(ValidationIssue::new(
                path,
                "invalid identifier: expected [A-Za-z_][A-Za-z0-9_]*",
            ));
        } else if matches!(self.0.as_str(), "true" | "false" | "null") {
            issues.push(ValidationIssue::new(
                path,
                "identifier must not be `true`, `false` or `null` (they are literals)",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_rejects_identifiers() {
        for ok in ["a", "_", "user_name", "A1", "x_9"] {
            assert!(Ident::is_valid(ok), "{ok}");
            assert!(Ident::new(ok).validate().is_empty(), "{ok}");
        }
        for bad in [
            "",
            "user-name",
            "1abc",
            "é",
            "a b",
            "a.b",
            "true",
            "false",
            "null",
        ] {
            assert!(!Ident::is_valid(bad), "{bad}");
            assert_eq!(Ident::new(bad).validate().len(), 1, "{bad}");
        }
        // keywords are only literal when the whole identifier matches
        assert!(Ident::is_valid("nullable"));
        assert!(Ident::is_valid("True"));
    }
}
