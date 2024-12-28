use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Clip, EdgeInsets, WidgetAsValue, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_on_foreground: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    margin: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_container: Option<bool>,
}

impl Widget for Card {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Card {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "card".to_string(),
            id: Uuid::now_v7().to_string(),
            color: None,
            shadow_color: None,
            surface_tint_color: None,
            elevation: None,
            border_on_foreground: None,
            margin: None,
            clip_behavior: None,
            child: None,
            semantic_container: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_shadow_color(mut self, shadow_color: Color) -> Self {
        self.shadow_color = Some(shadow_color.hex);
        self
    }

    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> Self {
        self.surface_tint_color = Some(surface_tint_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_border_on_foreground(mut self, border_on_foreground: bool) -> Self {
        self.border_on_foreground = Some(border_on_foreground);
        self
    }

    pub fn with_margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(child.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_semantic_container(mut self, semantic_container: bool) -> Self {
        self.semantic_container = Some(semantic_container);
        self
    }
}