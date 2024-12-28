use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::alignment_geometry::AlignmentGeometry;
use crate::sduix::flutter::mirai::border::Border;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::EdgeInsets;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogTheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment: Option<AlignmentGeometry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_color: Option<String>,
}

impl DialogTheme {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            shape: None,
            alignment: None,
            title_text_style: None,
            content_text_style: None,
            actions_padding: None,
            icon_color: None,
        }
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
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

    pub fn with_shape(mut self, shape: Border) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn with_alignment(mut self, alignment: AlignmentGeometry) -> Self {
        self.alignment = Some(alignment);
        self
    }

    pub fn with_title_text_style(mut self, title_text_style: TextStyle) -> Self {
        self.title_text_style = Some(title_text_style);
        self
    }

    pub fn with_content_text_style(mut self, content_text_style: TextStyle) -> Self {
        self.content_text_style = Some(content_text_style);
        self
    }

    pub fn with_actions_padding(mut self, actions_padding: EdgeInsets) -> Self {
        self.actions_padding = Some(actions_padding);
        self
    }

    pub fn with_icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = Some(icon_color.hex);
        self
    }
}