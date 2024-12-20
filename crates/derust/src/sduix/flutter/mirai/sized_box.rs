use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizedBox {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    width: Option<f64>,
    height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
}

impl Widget for SizedBox {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl SizedBox {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "sizedBox".to_string(),
            id: Uuid::now_v7().to_string(),
            width: None,
            height: None,
            child: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_child(mut self, widget: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(widget.as_value(tags)?);
        Ok(self)
    }
}
