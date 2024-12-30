use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::text_span::TextSpan;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{
    TextAlign, TextDirection, TextOverflow, TextWidthBasis, Widget,
};
use crate::sduix::Color;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Text {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    soft_wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_scale_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_lines: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantics_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selection_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_align: Option<TextAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_direction: Option<TextDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overflow: Option<TextOverflow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_width_basis: Option<TextWidthBasis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<TextSpan>>,
}

impl Widget for Text {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Text {
    pub fn new<S: Clone>(_context: &AppContext<S>, text: &str) -> Self {
        Self {
            widget_type: "text".to_string(),
            id: Uuid::now_v7().to_string(),
            data: text.to_string(),
            soft_wrap: None,
            text_scale_factor: None,
            max_lines: None,
            semantics_label: None,
            selection_color: None,
            text_align: None,
            text_direction: None,
            overflow: None,
            text_width_basis: None,
            style: None,
            children: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = Some(soft_wrap);
        self
    }

    pub fn with_text_scale_factor(mut self, text_scale_factor: f64) -> Self {
        self.text_scale_factor = Some(text_scale_factor);
        self
    }

    pub fn with_max_lines(mut self, max_lines: u64) -> Self {
        self.max_lines = Some(max_lines);
        self
    }

    pub fn with_semantics_label(mut self, semantics_label: &str) -> Self {
        self.semantics_label = Some(semantics_label.to_string());
        self
    }

    pub fn with_selection_color(mut self, selection_color: Color) -> Self {
        self.selection_color = Some(selection_color.hex);
        self
    }

    pub fn with_text_align(mut self, text_align: TextAlign) -> Self {
        self.text_align = Some(text_align);
        self
    }

    pub fn with_text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }

    pub fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = Some(overflow);
        self
    }

    pub fn with_text_width_basis(mut self, text_width_basis: TextWidthBasis) -> Self {
        self.text_width_basis = Some(text_width_basis);
        self
    }

    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_children(mut self, children: Vec<TextSpan>) -> Self {
        self.children = Some(children);
        self
    }
}
