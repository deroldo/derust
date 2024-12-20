use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{CrossAxisAlignment, MainAxisAlignment, MainAxisSize, TextDirection, VerticalDirection, Widget, WidgetsAsValue};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    main_axis_alignment: Option<MainAxisAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cross_axis_alignment: Option<CrossAxisAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    main_axis_size: Option<MainAxisSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_direction: Option<TextDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_direction: Option<VerticalDirection>,
    children: Vec<Value>,
}

impl Widget for Column {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Column {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        children: Vec<impl Widget>,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "column".to_string(),
            id: Uuid::now_v7().to_string(),
            main_axis_alignment: None,
            cross_axis_alignment: None,
            main_axis_size: None,
            text_direction: None,
            vertical_direction: None,
            children: children.as_values(tags)?,
        })
    }

    pub fn with_main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = Some(main_axis_alignment);
        self
    }

    pub fn with_cross_axis_alignment(mut self, cross_axis_alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = Some(cross_axis_alignment);
        self
    }

    pub fn with_main_axis_size(mut self, main_axis_size: MainAxisSize) -> Self {
        self.main_axis_size = Some(main_axis_size);
        self
    }

    pub fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }

    pub fn with_vertical_direction(mut self, vertical_direction: VerticalDirection) -> Self {
        self.vertical_direction = Some(vertical_direction);
        self
    }
}