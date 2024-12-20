use serde::Serialize;
use crate::sduix::flutter::mirai::offset::Offset;
use crate::sduix::flutter::mirai::widget::RectType;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    rect_type: RectType,
    #[serde(skip_serializing_if = "Option::is_none")]
    center: Option<Offset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    a: Option<Offset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    b: Option<Offset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    left: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
}

impl Rect {

}