use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::widget::BorderStyle;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderSide {
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    width: String,
    stroke_align: String,
    border_style: BorderStyle,
}

impl BorderSide {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        width: String,
        stroke_align: String,
        border_style: BorderStyle,
    ) -> Self {
        Self {
            color: None,
            width,
            stroke_align,
            border_style,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }
}