use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::rect::Rect;
use crate::sduix::flutter::mirai::widget::{Alignment, BoxFit, DecorationImageType, FilterQuality, ImageRepeat};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecorationImage {
    src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invert_colors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_anti_alias: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_text_direction: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_quality: Option<FilterQuality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat: Option<ImageRepeat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alignment: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fit: Option<BoxFit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_type: Option<DecorationImageType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_slice: Option<Rect>,
}

impl DecorationImage {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        src: &str,
    ) -> Self {
        Self {
            src: src.to_string(),
            scale: None,
            opacity: None,
            invert_colors: None,
            is_anti_alias: None,
            match_text_direction: None,
            filter_quality: None,
            repeat: None,
            alignment: None,
            fit: None,
            image_type: None,
            center_slice: None,
        }
    }

    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn with_invert_colors(mut self, invert_colors: bool) -> Self {
        self.invert_colors = Some(invert_colors);
        self
    }

    pub fn with_is_anti_alias(mut self, is_anti_alias: bool) -> Self {
        self.is_anti_alias = Some(is_anti_alias);
        self
    }

    pub fn with_match_text_direction(mut self, match_text_direction: bool) -> Self {
        self.match_text_direction = Some(match_text_direction);
        self
    }

    pub fn with_filter_quality(mut self, filter_quality: FilterQuality) -> Self {
        self.filter_quality = Some(filter_quality);
        self
    }

    pub fn with_repeat(mut self, repeat: ImageRepeat) -> Self {
        self.repeat = Some(repeat);
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    pub fn with_fit(mut self, fit: BoxFit) -> Self {
        self.fit = Some(fit);
        self
    }

    pub fn with_image_type(mut self, image_type: DecorationImageType) -> Self {
        self.image_type = Some(image_type);
        self
    }

    pub fn with_center_slice(mut self, center_slice: Rect) -> Self {
        self.center_slice = Some(center_slice);
        self
    }
}