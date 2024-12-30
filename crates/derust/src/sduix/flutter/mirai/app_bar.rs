use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Widget, WidgetAsValue, WidgetsAsValue};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBar {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    leading: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    background_color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toolbar_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    leading_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrolled_under_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_title: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toolbar_opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom_opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<bool>,
}

impl Widget for AppBar {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl AppBar {
    pub fn new<S: Clone>(context: &AppContext<S>) -> Self {
        Self {
            widget_type: "appBar".to_string(),
            id: Uuid::now_v7().to_string(),
            title: None,
            actions: None,
            bottom: None,
            leading: None,
            shadow_color: None,
            background_color: context.theme().primary_color.hex.clone(),
            foreground_color: None,
            surface_tint_color: None,
            toolbar_height: None,
            leading_width: None,
            elevation: None,
            scrolled_under_elevation: None,
            title_spacing: None,
            center_title: None,
            toolbar_opacity: None,
            bottom_opacity: None,
            primary: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_title(mut self, title: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.title = Some(title.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_bottom(mut self, bottom: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.bottom = Some(bottom.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_leading(
        mut self,
        leading: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        self.leading = Some(leading.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_shadow_color(mut self, shadow_color: Color) -> Self {
        self.shadow_color = Some(shadow_color.hex);
        self
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = background_color.hex;
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: Color) -> Self {
        self.foreground_color = Some(foreground_color.hex);
        self
    }

    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> Self {
        self.surface_tint_color = Some(surface_tint_color.hex);
        self
    }

    pub fn with_toolbar_height(mut self, toolbar_height: f64) -> Self {
        self.toolbar_height = Some(toolbar_height);
        self
    }

    pub fn with_leading_width(mut self, leading_width: f64) -> Self {
        self.leading_width = Some(leading_width);
        self
    }

    pub fn with_bottom_opacity(mut self, bottom_opacity: f64) -> Self {
        self.bottom_opacity = Some(bottom_opacity);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> Self {
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    pub fn with_title_spacing(mut self, title_spacing: f64) -> Self {
        self.title_spacing = Some(title_spacing);
        self
    }

    pub fn with_center_title(mut self, center_title: bool) -> Self {
        self.center_title = Some(center_title);
        self
    }

    pub fn with_primary(mut self, primary: bool) -> Self {
        self.primary = Some(primary);
        self
    }

    pub fn with_toolbar_opacity(mut self, toolbar_opacity: f64) -> Self {
        self.toolbar_opacity = Some(toolbar_opacity);
        self
    }

    pub fn with_actions(
        mut self,
        actions: Vec<impl Widget>,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        self.actions = Some(actions.widgets_as_values(tags)?);
        Ok(self)
    }
}
