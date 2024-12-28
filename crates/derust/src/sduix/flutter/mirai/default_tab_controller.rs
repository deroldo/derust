use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{WidgetAsValue, Widget};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultTabController {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    length: i64,
    child: Value,
}

impl Widget for DefaultTabController {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl DefaultTabController {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        length: i64,
        child: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "defaultBottomNavigationController".to_string(),
            id: Uuid::now_v7().to_string(),
            length,
            child: child.widget_as_value(tags)?,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }
}
