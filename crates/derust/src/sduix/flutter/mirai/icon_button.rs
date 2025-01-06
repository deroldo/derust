use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::box_constraints::BoxConstraints;
use crate::sduix::flutter::mirai::button_style::ButtonStyle;
use crate::sduix::flutter::mirai::icon::Icon;
use crate::sduix::flutter::mirai::widget::{
    Action, ActionAsValue, Alignment, EdgeInsets, Widget, WidgetAsValue,
};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconButton {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    splash_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highlight_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    splash_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_pressed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    constraints: Option<BoxConstraints>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<ButtonStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_icon: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<Value>,
}

impl Widget for IconButton {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl IconButton {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            widget_type: "iconButton".to_string(),
            id: Uuid::now_v7().to_string(),
            icon_size: None,
            padding: None,
            alignment: None,
            splash_radius: None,
            color: None,
            focus_color: None,
            hover_color: None,
            highlight_color: None,
            splash_color: None,
            disabled_color: None,
            on_pressed: None,
            autofocus: None,
            tooltip: None,
            enable_feedback: None,
            constraints: None,
            style: None,
            is_selected: None,
            selected_icon: None,
            icon: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_icon_size(mut self, icon_size: f64) -> Self {
        self.icon_size = Some(icon_size);
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    pub fn with_splash_radius(mut self, splash_radius: f64) -> Self {
        self.splash_radius = Some(splash_radius);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
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

    pub fn with_highlight_color(mut self, highlight_color: Color) -> Self {
        self.highlight_color = Some(highlight_color.hex);
        self
    }

    pub fn with_splash_color(mut self, splash_color: Color) -> Self {
        self.splash_color = Some(splash_color.hex);
        self
    }

    pub fn with_disabled_color(mut self, disabled_color: Color) -> Self {
        self.disabled_color = Some(disabled_color.hex);
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

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_tooltip(mut self, tooltip: &str) -> Self {
        self.tooltip = Some(tooltip.to_string());
        self
    }

    pub fn with_enable_feedback(mut self, enable_feedback: bool) -> Self {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    pub fn with_constraints(mut self, constraints: BoxConstraints) -> Self {
        self.constraints = Some(constraints);
        self
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_is_selected(mut self, is_selected: bool) -> Self {
        self.is_selected = Some(is_selected);
        self
    }

    pub fn with_selected_icon(
        mut self,
        selected_icon: Icon,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        self.selected_icon = Some(selected_icon.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_icon(mut self, icon: Icon, tags: &HttpTags) -> Result<Self, HttpError> {
        self.icon = Some(icon.widget_as_value(tags)?);
        Ok(self)
    }
}
