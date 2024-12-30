use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::offset::Offset;
use crate::sduix::flutter::mirai::widget::BlurStyle;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxShadow {
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blur_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<Offset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spread_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blur_style: Option<BlurStyle>,
}

impl BoxShadow {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            color: None,
            blur_radius: None,
            offset: None,
            spread_radius: None,
            blur_style: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_blur_radius(mut self, blur_radius: f64) -> Self {
        self.blur_radius = Some(blur_radius);
        self
    }

    pub fn with_offset(mut self, offset: Offset) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_spread_radius(mut self, spread_radius: f64) -> Self {
        self.spread_radius = Some(spread_radius);
        self
    }

    pub fn with_blur_style(mut self, blur_style: BlurStyle) -> Self {
        self.blur_style = Some(blur_style);
        self
    }
}
