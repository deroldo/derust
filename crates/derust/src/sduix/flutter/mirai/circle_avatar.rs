use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Widget, WidgetAsValue};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircleAvatar {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_background_image_error: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_foreground_image_error: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_radius: Option<f64>,
}

impl Widget for CircleAvatar {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl CircleAvatar {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "card".to_string(),
            id: Uuid::now_v7().to_string(),
            child: None,
            background_color: None,
            background_image: None,
            foreground_image: None,
            on_background_image_error: None,
            on_foreground_image_error: None,
            foreground_color: None,
            radius: None,
            min_radius: None,
            max_radius: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(child.as_value(tags)?);
        Ok(self)
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_foreground_image(mut self, foreground_image: &str) -> Self {
        self.foreground_image = Some(foreground_image.to_string());
        self
    }

    pub fn with_background_image(mut self, background_image: &str) -> Self {
        self.background_image = Some(background_image.to_string());
        self
    }

    pub fn with_on_background_image_error(mut self, on_background_image_error: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_background_image_error = Some(on_background_image_error.as_value(tags)?);
        Ok(self)
    }

    pub fn with_on_foreground_image_error(mut self, on_foreground_image_error: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_foreground_image_error = Some(on_foreground_image_error.as_value(tags)?);
        Ok(self)
    }

    pub fn with_foreground_color(mut self, foreground_color: Color) -> Self {
        self.foreground_color = Some(foreground_color.hex);
        self
    }

    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn with_min_radius(mut self, min_radius: f64) -> Self {
        self.min_radius = Some(min_radius);
        self
    }

    pub fn with_max_radius(mut self, max_radius: f64) -> Self {
        self.max_radius = Some(max_radius);
        self
    }
}