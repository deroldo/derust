use serde::Serialize;
use crate::httpx::AppContext;
use crate::sduix::Color;
use crate::sduix::flutter::mirai::border::Border;
use crate::sduix::flutter::mirai::border_radius::BorderRadius;
use crate::sduix::flutter::mirai::box_shadow::BoxShadow;
use crate::sduix::flutter::mirai::decoration_image::DecorationImage;
use crate::sduix::flutter::mirai::gradient::Gradient;
use crate::sduix::flutter::mirai::widget::{BlendMode, BoxShape};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxDecoration {
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_blend_mode: Option<BlendMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box_shadow: Option<Vec<BoxShadow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<BoxShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_radius: Option<BorderRadius>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<DecorationImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gradient: Option<Gradient>,
}

impl BoxDecoration {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            color: None,
            background_blend_mode: None,
            box_shadow: None,
            shape: None,
            border: None,
            border_radius: None,
            image: None,
            gradient: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_background_blend_mode(mut self, background_blend_mode: BlendMode) -> Self {
        self.background_blend_mode = Some(background_blend_mode);
        self
    }

    pub fn with_box_shadow(mut self, box_shadow: Vec<BoxShadow>) -> Self {
        self.box_shadow = Some(box_shadow);
        self
    }

    pub fn with_shape(mut self, shape: BoxShape) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn with_border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    pub fn with_border_radius(mut self, border_radius: BorderRadius) -> Self {
        self.border_radius = Some(border_radius);
        self
    }

    pub fn with_image(mut self, image: DecorationImage) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_gradient(mut self, gradient: Gradient) -> Self {
        self.gradient = Some(gradient);
        self
    }
}