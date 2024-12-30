use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Center {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    width_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
}

impl Widget for Center {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Center {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            widget_type: "card".to_string(),
            id: Uuid::now_v7().to_string(),
            width_factor: None,
            height_factor: None,
            child: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_width_factor(mut self, width_factor: f64) -> Self {
        self.width_factor = Some(width_factor);
        self
    }

    pub fn with_height_factor(mut self, height_factor: f64) -> Self {
        self.height_factor = Some(height_factor);
        self
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(child.widget_as_value(tags)?);
        Ok(self)
    }
}
