use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{
    Action, ActionAsValue, FloatingActionButtonType, Widget, WidgetAsValue,
};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatingActionButton {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_pressed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    button_type: Option<FloatingActionButtonType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    splash_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_text_style: Option<TextStyle>,
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
    extended_icon_label_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hero_tag: Option<Value>,
    child: Value,
}

impl Widget for FloatingActionButton {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl FloatingActionButton {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        child: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "floatingActionButton".to_string(),
            id: Uuid::now_v7().to_string(),
            on_pressed: None,
            text_style: None,
            button_type: None,
            autofocus: None,
            icon: None,
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
            tooltip: None,
            hero_tag: None,
            child: child.widget_as_value(tags)?,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_on_pressed(
        mut self,
        on_pressed: impl Action,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        self.on_pressed = Some(on_pressed.action_as_value(tags)?);
        Ok(self)
    }

    pub fn with_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn with_button_type(mut self, button_type: FloatingActionButtonType) -> Self {
        self.button_type = Some(button_type);
        self
    }

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_icon(mut self, icon: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.icon = Some(icon.widget_as_value(tags)?);
        Ok(self)
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

    pub fn with_tooltip(mut self, tooltip: &str) -> Self {
        self.tooltip = Some(tooltip.to_string());
        self
    }

    pub fn with_hero_tag(
        mut self,
        hero_tag: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        self.hero_tag = Some(hero_tag.widget_as_value(tags)?);
        Ok(self)
    }
}
