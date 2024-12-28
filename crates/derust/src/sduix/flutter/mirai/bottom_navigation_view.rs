use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{WidgetAsValue, WidgetsAsValue, Widget};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BottomNavigationView {
    children: Vec<Value>,
}

impl BottomNavigationView {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            children: vec![],
        }
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.children.push(child.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_children(mut self, children: Vec<impl Widget>, tags: &HttpTags) -> Result<Self, HttpError> {
        children.widgets_as_values(tags)?.into_iter().for_each(|child| self.children.push(child));
        Ok(self)
    }
}