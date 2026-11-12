use super::ident::Ident;
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::{Serialize, Serializer};
use std::fmt;

/// First element of a [`StatePath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Root {
    /// A screen state key or a local variable (`user`, `item`, `todos`).
    Key(Ident),
    /// A global state key; always printed `global.key` (never `global['key']`).
    Global(Ident),
    /// `params`: the parameters received by a screen opened with `navigate`.
    Params,
    /// `result`: the value returned to an `onResult` callback.
    Result,
    /// `response`: the network response inside `onSuccess`/`onError`/`onFinally`.
    Response,
    /// `event`: the event of an `onChanged`/`onSelected` callback.
    Event,
}

/// Step after the [`Root`] of a [`StatePath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// `.field`
    Field(Ident),
    /// `[n]` with a literal, non negative integer.
    Index(usize),
}

/// A path into the state, as accepted by `{"$state": ".."}`: a [`Root`] followed by `.field`
/// and `[n]` segments. It has **no operators** (use [`Expr`](super::Expr) for those).
///
/// ```
/// use derust::sduix::stacx::v1::state::StatePath;
///
/// assert_eq!(StatePath::global("token").to_string(), "global.token");
/// assert_eq!(StatePath::key("items").index(0).field("id").to_string(), "items[0].id");
/// assert_eq!(StatePath::response().field("body").to_string(), "response.body");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StatePath {
    root: Root,
    segments: Vec<PathSegment>,
}

impl StatePath {
    fn from_root(root: Root) -> Self {
        Self {
            root,
            segments: Vec::new(),
        }
    }

    /// Screen state key or local variable.
    pub fn key(name: impl Into<Ident>) -> Self {
        Self::from_root(Root::Key(name.into()))
    }

    /// Global state key (`global.name`).
    pub fn global(name: impl Into<Ident>) -> Self {
        Self::from_root(Root::Global(name.into()))
    }

    /// `params`
    pub fn params() -> Self {
        Self::from_root(Root::Params)
    }

    /// `result`
    pub fn result() -> Self {
        Self::from_root(Root::Result)
    }

    /// `response`
    pub fn response() -> Self {
        Self::from_root(Root::Response)
    }

    /// `event`
    pub fn event() -> Self {
        Self::from_root(Root::Event)
    }

    /// Appends `.name`.
    pub fn field(mut self, name: impl Into<Ident>) -> Self {
        self.segments.push(PathSegment::Field(name.into()));
        self
    }

    /// Appends `[index]`.
    pub fn index(mut self, index: usize) -> Self {
        self.segments.push(PathSegment::Index(index));
        self
    }

    /// The root of the path.
    pub fn root(&self) -> &Root {
        &self.root
    }

    /// The segments after the root.
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }
}

impl fmt::Display for StatePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.root {
            Root::Key(name) => write!(f, "{name}")?,
            Root::Global(name) => write!(f, "global.{name}")?,
            Root::Params => f.write_str("params")?,
            Root::Result => f.write_str("result")?,
            Root::Response => f.write_str("response")?,
            Root::Event => f.write_str("event")?,
        }
        for segment in &self.segments {
            match segment {
                PathSegment::Field(name) => write!(f, ".{name}")?,
                PathSegment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

impl Serialize for StatePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl Validate for StatePath {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        match &self.root {
            Root::Key(name) | Root::Global(name) => name.validate_at(&path.key("root"), issues),
            Root::Params | Root::Result | Root::Response | Root::Event => {}
        }
        for (index, segment) in self.segments.iter().enumerate() {
            if let PathSegment::Field(name) = segment {
                name.validate_at(&path.key("segments").index(index), issues);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_root_and_segment() {
        assert_eq!(StatePath::global("token").to_string(), "global.token");
        assert_eq!(
            StatePath::key("user").field("name").to_string(),
            "user.name"
        );
        assert_eq!(StatePath::key("items").index(0).to_string(), "items[0]");
        assert_eq!(StatePath::params().field("name").to_string(), "params.name");
        assert_eq!(StatePath::result().to_string(), "result");
        assert_eq!(
            StatePath::response()
                .field("body")
                .field("items")
                .to_string(),
            "response.body.items"
        );
        assert_eq!(StatePath::event().field("value").to_string(), "event.value");
        assert_eq!(
            StatePath::global("a")
                .field("b")
                .index(2)
                .field("c")
                .to_string(),
            "global.a.b[2].c"
        );
    }

    #[test]
    fn invalid_identifiers_are_reported_with_their_location() {
        for bad in ["user-name", "1abc", "é", "true", ""] {
            let issues = StatePath::key(bad).validate();
            assert_eq!(issues.len(), 1, "{bad}");
            assert_eq!(issues[0].path, "$.root", "{bad}");
        }
        let issues = StatePath::global("null").validate();
        assert_eq!(issues[0].path, "$.root");
        let issues = StatePath::key("a").field("b").field("a-b").validate();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path, "$.segments[1]");
        assert!(StatePath::params()
            .field("ok")
            .index(3)
            .validate()
            .is_empty());
    }

    #[test]
    fn serializes_as_its_source_text() {
        let value = serde_json::to_value(StatePath::global("token")).unwrap();
        assert_eq!(value, serde_json::json!("global.token"));
    }
}
