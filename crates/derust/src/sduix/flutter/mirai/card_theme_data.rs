use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::border::Border;
use crate::sduix::flutter::mirai::widget::{Clip, EdgeInsets};
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardThemeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    margin: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Border>,
}

impl CardThemeData {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            clip_behavior: None,
            color: None,
            shadow_color: None,
            surface_tint_color: None,
            elevation: None,
            margin: None,
            shape: None,
        }
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
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

    pub fn with_margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn with_shape(mut self, shape: Border) -> Self {
        self.shape = Some(shape);
        self
    }
}