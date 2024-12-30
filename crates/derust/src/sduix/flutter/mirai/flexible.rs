use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{FlexFit, Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flexible {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flex: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fit: Option<FlexFit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
}

impl Widget for Flexible {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Flexible {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            widget_type: "flexible".to_string(),
            id: Uuid::now_v7().to_string(),
            flex: None,
            fit: None,
            child: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_flex(mut self, flex: i64) -> Self {
        self.flex = Some(flex);
        self
    }

    pub fn with_fit(mut self, fit: FlexFit) -> Self {
        self.fit = Some(fit);
        self
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(child.widget_as_value(tags)?);
        Ok(self)
    }
}
