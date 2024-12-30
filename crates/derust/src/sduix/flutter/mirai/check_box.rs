use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::widget::{MaterialColor, Widget};
use crate::sduix::Color;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckBox {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    check_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tristate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    splash_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fill_color: Option<MaterialColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overlay_color: Option<MaterialColor>,
}

impl Widget for CheckBox {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl CheckBox {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            widget_type: "card".to_string(),
            id: Uuid::now_v7().to_string(),
            value: None,
            active_color: None,
            check_color: None,
            tristate: None,
            focus_color: None,
            hover_color: None,
            splash_radius: None,
            autofocus: None,
            is_error: None,
            fill_color: None,
            overlay_color: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_value(mut self, value: bool) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_active_color(mut self, active_color: Color) -> Self {
        self.active_color = Some(active_color.hex);
        self
    }

    pub fn with_check_color(mut self, check_color: Color) -> Self {
        self.check_color = Some(check_color.hex);
        self
    }

    pub fn with_tristate(mut self, tristate: bool) -> Self {
        self.tristate = Some(tristate);
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

    pub fn with_splash_radius(mut self, splash_radius: f64) -> Self {
        self.splash_radius = Some(splash_radius);
        self
    }

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_is_error(mut self, is_error: bool) -> Self {
        self.is_error = Some(is_error);
        self
    }

    pub fn with_fill_color(mut self, fill_color: MaterialColor) -> Self {
        self.fill_color = Some(fill_color);
        self
    }

    pub fn with_overlay_color(mut self, overlay_color: MaterialColor) -> Self {
        self.overlay_color = Some(overlay_color);
        self
    }
}
