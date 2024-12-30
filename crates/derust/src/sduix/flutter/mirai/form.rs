use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{AutovalidateMode, Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Form {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    child: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    autovalidate_mode: Option<AutovalidateMode>,
}

impl Widget for Form {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Form {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        child: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "form".to_string(),
            id: Uuid::now_v7().to_string(),
            child: child.widget_as_value(tags)?,
            autovalidate_mode: None,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_autovalidate_mode(mut self, autovalidate_mode: AutovalidateMode) -> Self {
        self.autovalidate_mode = Some(autovalidate_mode);
        self
    }
}
