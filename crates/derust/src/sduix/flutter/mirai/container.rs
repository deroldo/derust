use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::box_constraints::BoxConstraints;
use crate::sduix::flutter::mirai::box_decoration::BoxDecoration;
use crate::sduix::flutter::mirai::widget::{Alignment, EdgeInsets, WidgetAsValue, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    margin: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decoration: Option<BoxDecoration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_decoration: Option<BoxDecoration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    constraints: Option<BoxConstraints>,
}

impl Widget for Container {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Container {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "container".to_string(),
            id: Uuid::now_v7().to_string(),
            child: None,
            padding: None,
            margin: None,
            color: None,
            width: None,
            height: None,
            alignment: None,
            decoration: None,
            foreground_decoration: None,
            constraints: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_child(mut self, widget: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(widget.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn with_margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
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

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    pub fn with_decoration(mut self, decoration: BoxDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }

    pub fn with_foreground_decoration(mut self, foreground_decoration: BoxDecoration) -> Self {
        self.foreground_decoration = Some(foreground_decoration);
        self
    }

    pub fn with_constraints(mut self, constraints: BoxConstraints) -> Self {
        self.constraints = Some(constraints);
        self
    }
}
