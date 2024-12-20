use serde::Serialize;
use crate::httpx::AppContext;
use crate::sduix::Color;
use crate::sduix::flutter::mirai::widget::{BorderSide, BorderStyle};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Border {
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_style: Option<BorderStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stroke_align: Option<f64>,
}

impl Border {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            color: None,
            border_style: None,
            width: None,
            stroke_align: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_border_style(mut self, border_style: BorderStyle) -> Self {
        self.border_style = Some(border_style);
        self
    }

    pub fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_stroke_align(mut self, stroke_align: BorderSide) -> Self {
        self.stroke_align = Some(stroke_align.get_value());
        self
    }
}