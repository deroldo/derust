use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSpan {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_tap: Option<Value>,
}

impl Widget for TextSpan {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl TextSpan {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        text_span: &str,
    ) -> Self {
        Self {
            widget_type: "textSpan".to_string(),
            id: Uuid::now_v7().to_string(),
            data: text_span.to_string(),
            style: None,
            on_tap: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_styled(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_on_tap(mut self, on_tap: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_tap = Some(on_tap.as_value(tags)?);
        Ok(self)
    }
}