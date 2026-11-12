use super::decl::{BindKey, GlobalStateBlock, OnInit, StateBlock};
use super::reactive::{IntoReactive, Reactive};
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::ser::{Serialize, SerializeMap, Serializer};

/// Properties every widget accepts besides its own: visibility, binding and state declarations.
///
/// Serialized right after the widget discriminator through [`Envelope::serialize_into`]; custom
/// widgets embed an `Envelope` and call it from their own `Serialize`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Envelope {
    /// `visible`.
    pub visible: Option<Reactive<bool>>,
    /// `enabled`.
    pub enabled: Option<Reactive<bool>>,
    /// `bind`.
    pub bind: Option<BindKey>,
    /// `state`.
    pub state: Option<StateBlock>,
    /// `globalState`.
    pub global_state: Option<GlobalStateBlock>,
    /// `onInit`.
    pub on_init: Option<OnInit>,
}

impl Envelope {
    /// Empty envelope.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when nothing would be serialized.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    /// Sets `visible`.
    pub fn visible(mut self, value: impl IntoReactive<bool>) -> Self {
        self.visible = Some(value.into_reactive());
        self
    }

    /// Sets `enabled`.
    pub fn enabled(mut self, value: impl IntoReactive<bool>) -> Self {
        self.enabled = Some(value.into_reactive());
        self
    }

    /// Sets `bind`.
    pub fn bind(mut self, key: BindKey) -> Self {
        self.bind = Some(key);
        self
    }

    /// Sets `state`.
    pub fn state(mut self, block: StateBlock) -> Self {
        self.state = Some(block);
        self
    }

    /// Sets `globalState`.
    pub fn global_state(mut self, block: GlobalStateBlock) -> Self {
        self.global_state = Some(block);
        self
    }

    /// Sets `onInit`.
    pub fn on_init(mut self, on_init: impl Into<OnInit>) -> Self {
        self.on_init = Some(on_init.into());
        self
    }

    /// Writes the present properties as entries of an object being serialized.
    pub fn serialize_into<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
        if let Some(value) = &self.visible {
            map.serialize_entry("visible", value)?;
        }
        if let Some(value) = &self.enabled {
            map.serialize_entry("enabled", value)?;
        }
        if let Some(value) = &self.bind {
            map.serialize_entry("bind", value)?;
        }
        if let Some(value) = &self.state {
            map.serialize_entry("state", value)?;
        }
        if let Some(value) = &self.global_state {
            map.serialize_entry("globalState", value)?;
        }
        if let Some(value) = &self.on_init {
            map.serialize_entry("onInit", value)?;
        }
        Ok(())
    }
}

impl Serialize for Envelope {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        self.serialize_into(&mut map)?;
        map.end()
    }
}

impl Validate for Envelope {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        self.visible.validate_at(&path.key("visible"), issues);
        self.enabled.validate_at(&path.key("enabled"), issues);
        self.bind.validate_at(&path.key("bind"), issues);
        self.state.validate_at(&path.key("state"), issues);
        self.global_state
            .validate_at(&path.key("globalState"), issues);
        self.on_init.validate_at(&path.key("onInit"), issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sduix::stacx::v1::state::{Expr, StatePath};
    use serde_json::json;

    #[test]
    fn empty_envelope_serializes_nothing() {
        let envelope = Envelope::new();
        assert!(envelope.is_empty());
        assert_eq!(serde_json::to_value(&envelope).unwrap(), json!({}));
    }

    #[test]
    fn full_envelope() {
        let envelope = Envelope::new()
            .visible(Expr::from(StatePath::key("n")).gt(0))
            .enabled(true)
            .bind(BindKey::global("name"))
            .state(StateBlock::new().with("n", 0))
            .global_state(GlobalStateBlock::new())
            .on_init(vec![]);
        assert!(!envelope.is_empty());
        assert_eq!(
            serde_json::to_value(&envelope).unwrap(),
            json!({
                "visible": {"$expr": "n > 0"},
                "enabled": true,
                "bind": "global.name",
                "state": {"n": 0},
                "globalState": {},
                "onInit": []
            })
        );
    }

    #[test]
    fn validation_paths() {
        let envelope = Envelope::new().bind(BindKey::screen(""));
        assert_eq!(envelope.validate()[0].path, "$.bind");
    }
}
