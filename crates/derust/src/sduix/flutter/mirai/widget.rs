use axum::http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use crate::httpx::{HttpError, HttpTags};

pub trait Widget: Clone + Serialize {
    fn get_id(&self) -> String;
    fn get_type(&self) -> String;
}

pub trait WidgetAsValue {
    fn as_value(&self, tags: &HttpTags) -> Result<Value, HttpError>;
}

impl<T: Widget> WidgetAsValue for T {
    fn as_value(&self, tags: &HttpTags) -> Result<Value, HttpError> {
        serde_json::to_value(self).map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to convert {} to value: {error}", self.get_type()),
                tags.clone(),
            )
        })
    }
}

pub trait WidgetsAsValue {
    fn as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError>;
}

impl<T: Widget> WidgetsAsValue for Vec<T> {
    fn as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError> {
        let mut values = vec![];

        for widget in self {
            values.push(widget.as_value(tags)?);
        }

        Ok(values)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MainAxisAlignment {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrossAxisAlignment {
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MainAxisSize {
    Min,
    Max,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextDirection {
    Ltr,
    Rtl,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
    Start,
    End,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextOverflow {
    Clip,
    Fade,
    Ellipsis,
    Visible,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextWidthBasis {
    Parent,
    LongestLine,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalDirection {
    Up,
    Down,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FloatingActionButtonLocation {
    StartTop,
    MiniStartTop,
    CenterTop,
    MiniCenterTop,
    EndTop,
    MiniEndTop,
    StartFloat,
    MiniStartFloat,
    CenterFloat,
    MiniCenterFloat,
    EndFloat,
    MiniEndFloat,
    StartDocked,
    MiniStartDocked,
    CenterDocked,
    MiniCenterDocked,
    EndDocked,
    MiniEndDocked,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollPhysics {
    Never,
    Bouncing,
    Clamping,
    Fixed,
    Page,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeInsets {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
}

impl EdgeInsets {
    pub fn all(
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
    ) -> Self {
        Self {
            left: Some(left),
            right: Some(right),
            top: Some(top),
            bottom: Some(bottom),
        }
    }

    pub fn left(
        left: f64,
    ) -> Self {
        Self {
            left: Some(left),
            right: None,
            top: None,
            bottom: None,
        }
    }

    pub fn right(
        right: f64,
    ) -> Self {
        Self {
            left: None,
            right: Some(right),
            top: None,
            bottom: None,
        }
    }

    pub fn top(
        top: f64,
    ) -> Self {
        Self {
            left: None,
            right: None,
            top: Some(top),
            bottom: None,
        }
    }

    pub fn bottom(
        bottom: f64,
    ) -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: Some(bottom),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DragStartBehavior {
    Start,
    Down,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollViewKeyboardDismissBehavior {
    Manual,
    OnDrag,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Clip {
    None,
    HardEdge,
    AntiAlias,
    AntiAliasWithSaveLayer,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FontWeight {
    W100,
    W200,
    W300,
    W400,
    W500,
    W600,
    W700,
    W800,
    W900,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextBaseline {
    Alphabetic,
    Ideographic,
}