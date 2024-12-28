use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::border_side::BorderSide;
use crate::sduix::flutter::mirai::rounded_rectangle_border::RoundedRectangleBorder;
use crate::sduix::flutter::mirai::size::Size;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::EdgeInsets;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum_size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    side: Option<BorderSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<RoundedRectangleBorder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_size: Option<f64>,
}

impl ButtonStyle {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            foreground_color: None,
            background_color: None,
            disabled_foreground_color: None,
            disabled_background_color: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_color: None,
            disabled_icon_color: None,
            elevation: None,
            text_style: None,
            padding: None,
            minimum_size: None,
            fixed_size: None,
            maximum_size: None,
            side: None,
            shape: None,
            enable_feedback: None,
            icon_size: None,
        }
    }

    pub fn with_foreground_color(mut self, foreground_color: Color) -> Self {
        self.foreground_color = Some(foreground_color.hex);
        self
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_disabled_foreground_color(mut self, disabled_foreground_color: Color) -> Self {
        self.disabled_foreground_color = Some(disabled_foreground_color.hex);
        self
    }

    pub fn with_disabled_background_color(mut self, disabled_background_color: Color) -> Self {
        self.disabled_background_color = Some(disabled_background_color.hex);
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

    pub fn with_icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = Some(icon_color.hex);
        self
    }

    pub fn with_disabled_icon_color(mut self, disabled_icon_color: Color) -> Self {
        self.disabled_icon_color = Some(disabled_icon_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn with_minimum_size(mut self, minimum_size: Size) -> Self {
        self.minimum_size = Some(minimum_size);
        self
    }

    pub fn with_fixed_size(mut self, fixed_size: Size) -> Self {
        self.fixed_size = Some(fixed_size);
        self
    }

    pub fn with_maximum_size(mut self, maximum_size: Size) -> Self {
        self.maximum_size = Some(maximum_size);
        self
    }

    pub fn with_side(mut self, side: BorderSide) -> Self {
        self.side = Some(side);
        self
    }

    pub fn with_shape(mut self, shape: RoundedRectangleBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn with_enable_feedback(mut self, enable_feedback: bool) -> Self {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    pub fn with_icon_size(mut self, icon_size: f64) -> Self {
        self.icon_size = Some(icon_size);
        self
    }
}