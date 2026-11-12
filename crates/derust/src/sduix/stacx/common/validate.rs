use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// Location of a value inside a screen, built lazily (only rendered when an issue is found).
#[derive(Debug, Clone, Copy)]
pub struct Path<'a> {
    parent: Option<&'a Path<'a>>,
    segment: Segment<'a>,
}

#[derive(Debug, Clone, Copy)]
enum Segment<'a> {
    Root(&'a str),
    Key(&'a str),
    Index(usize),
}

impl<'a> Path<'a> {
    /// Root of a path (e.g. `"$"`).
    pub fn root(name: &'a str) -> Self {
        Self {
            parent: None,
            segment: Segment::Root(name),
        }
    }

    /// Child path for an object key.
    pub fn key<'b>(&'b self, key: &'b str) -> Path<'b> {
        Path {
            parent: Some(self),
            segment: Segment::Key(key),
        }
    }

    /// Child path for an array index.
    pub fn index(&self, index: usize) -> Path<'_> {
        Path {
            parent: Some(self),
            segment: Segment::Index(index),
        }
    }
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(parent) = self.parent {
            parent.fmt(f)?;
        }
        match self.segment {
            Segment::Root(name) => f.write_str(name),
            Segment::Key(key) => write!(f, ".{key}"),
            Segment::Index(index) => write!(f, "[{index}]"),
        }
    }
}

/// A problem found while validating a screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    /// JSON-like location of the problem (e.g. `$.children[1].padding.left`).
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl ValidationIssue {
    /// Creates an issue located at `path`.
    pub fn new(path: &Path<'_>, message: impl Into<String>) -> Self {
        Self {
            path: path.to_string(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Checks that a tree can be faithfully serialized and respects the constraints found in Stac.
pub trait Validate {
    /// Appends the issues found in `self` (and its children) to `issues`.
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>);

    /// All issues found, with paths relative to `$`.
    fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        self.validate_at(&Path::root("$"), &mut issues);
        issues
    }

    /// Checks that depend on context, run over the serialized JSON of the screen (what the
    /// client actually sees). The default does nothing; `v1` widgets and actions override it.
    fn validate_serialized(&self, _json: &serde_json::Value, _report: &mut ValidationReport) {}
}

/// Outcome of the checks over the serialized screen: `errors` block the response, `warnings`
/// are only logged.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ValidationReport {
    /// Problems that make the screen invalid.
    pub errors: Vec<ValidationIssue>,
    /// Suspicious but accepted constructs.
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there are no errors (warnings do not count).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Validate for f64 {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if !self.is_finite() {
            issues.push(ValidationIssue::new(
                path,
                "number is not finite and cannot be represented in JSON",
            ));
        }
    }
}

macro_rules! validate_noop {
    ($($ty:ty),* $(,)?) => {
        $(impl Validate for $ty {
            fn validate_at(&self, _path: &Path<'_>, _issues: &mut Vec<ValidationIssue>) {}
        })*
    };
}

validate_noop!(
    bool,
    i64,
    String,
    serde_json::Value,
    serde_json::Map<String, serde_json::Value>
);

impl<T: Validate> Validate for Option<T> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if let Some(value) = self {
            value.validate_at(path, issues);
        }
    }
}

impl<T: Validate + ?Sized> Validate for Box<T> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        (**self).validate_at(path, issues);
    }
}

impl<T: Validate> Validate for Vec<T> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        for (index, value) in self.iter().enumerate() {
            value.validate_at(&path.index(index), issues);
        }
    }
}

impl<K: fmt::Display, V: Validate> Validate for BTreeMap<K, V> {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        for (key, value) in self {
            let key = key.to_string();
            value.validate_at(&path.key(&key), issues);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_numbers_are_reported_with_path() {
        let values = vec![1.0, f64::NAN];
        let issues = values.validate();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$[1]");
    }

    #[test]
    fn nested_paths_render_keys_and_indexes() {
        let root = Path::root("$");
        let key = root.key("children");
        let index = key.index(2);
        assert_eq!(index.key("child").to_string(), "$.children[2].child");
    }
}
