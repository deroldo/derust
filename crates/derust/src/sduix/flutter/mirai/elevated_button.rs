use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::button_style::ButtonStyle;
use crate::sduix::flutter::mirai::widget::{Action, ActionAsValue, Clip, Widget, WidgetAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedButton {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_pressed: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<ButtonStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child: Option<Value>,
}

impl Widget for ElevatedButton {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl ElevatedButton {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "elevatedButton".to_string(),
            id: Uuid::now_v7().to_string(),
            on_pressed: None,
            style: None,
            autofocus: None,
            clip_behavior: None,
            child: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_on_pressed(mut self, on_pressed: impl Action, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_pressed = Some(on_pressed.action_as_value(tags)?);
        Ok(self)
    }

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    pub fn with_child(mut self, child: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.child = Some(child.widget_as_value(tags)?);
        Ok(self)
    }
}