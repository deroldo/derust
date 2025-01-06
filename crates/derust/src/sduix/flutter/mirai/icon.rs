use crate::httpx::{AppContext, HttpError};
use crate::sduix::flutter::mirai::widget::{IconType, TextDirection, Widget};
use crate::sduix::Color;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    icon: String,
    icon_type: IconType,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_direction: Option<TextDirection>,
}

impl Widget for Icon {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Icon {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        icon: &str,
        icon_type: IconType,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "icon".to_string(),
            id: Uuid::now_v7().to_string(),
            icon: icon.to_string(),
            icon_type,
            size: None,
            color: None,
            semantic_label: None,
            text_direction: None,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_semantic_label(mut self, semantic_label: &str) -> Self {
        self.semantic_label = Some(semantic_label.to_string());
        self
    }

    pub fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }
}
