use crate::sduix::flutter::mirai::widget::{FontStyle, FontWeight, TextBaseline};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    inherit: Option<bool>,
    color: Option<String>,
    background_color: Option<String>,
    style_from_theme: Option<String>,
    font_size: Option<f64>,
    letter_spacing: Option<f64>,
    word_spacing: Option<f64>,
    height: Option<f64>,
    font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    font_weight: Option<FontWeight>,
    font_style: Option<FontStyle>,
    text_baseline: Option<TextBaseline>,
}
