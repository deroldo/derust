use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::EdgeInsets;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatingActionButtonThemeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    splash_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highlight_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_icon_label_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_text_style: Option<TextStyle>,
}

impl FloatingActionButtonThemeData {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            background_color: None,
            foreground_color: None,
            focus_color: None,
            hover_color: None,
            splash_color: None,
            extended_text_style: None,
            elevation: None,
            focus_elevation: None,
            hover_elevation: None,
            disabled_elevation: None,
            highlight_elevation: None,
            extended_icon_label_spacing: None,
            enable_feedback: None,
            icon_size: None,
            extended_padding: None,
        }
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: Color) -> Self {
        self.foreground_color = Some(foreground_color.hex);
        self
    }

    pub fn with_focus_color(mut self, focus_color: Color) -> Self {
        self.focus_color = Some(focus_color.hex);
        self
    }

    pub fn with_hover_color(mut self, hover_color: Color) -> Self {
        self.hover_color = Some(hover_color.hex);
        self
    }

    pub fn with_splash_color(mut self, splash_color: Color) -> Self {
        self.splash_color = Some(splash_color.hex);
        self
    }

    pub fn with_extended_text_style(mut self, extended_text_style: TextStyle) -> Self {
        self.extended_text_style = Some(extended_text_style);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_focus_elevation(mut self, focus_elevation: f64) -> Self {
        self.focus_elevation = Some(focus_elevation);
        self
    }

    pub fn with_hover_elevation(mut self, hover_elevation: f64) -> Self {
        self.hover_elevation = Some(hover_elevation);
        self
    }

    pub fn with_disabled_elevation(mut self, disabled_elevation: f64) -> Self {
        self.disabled_elevation = Some(disabled_elevation);
        self
    }

    pub fn with_highlight_elevation(mut self, highlight_elevation: f64) -> Self {
        self.highlight_elevation = Some(highlight_elevation);
        self
    }

    pub fn with_extended_icon_label_spacing(mut self, extended_icon_label_spacing: f64) -> Self {
        self.extended_icon_label_spacing = Some(extended_icon_label_spacing);
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

    pub fn with_extended_padding(mut self, extended_padding: EdgeInsets) -> Self {
        self.extended_padding = Some(extended_padding);
        self
    }
}
