use axum::http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use crate::httpx::{HttpError, HttpTags};

pub trait Widget: Clone + Serialize {
    fn get_id(&self) -> String;
    fn get_type(&self) -> String;
}

pub trait WidgetAsValue {
    fn widget_as_value(&self, tags: &HttpTags) -> Result<Value, HttpError>;
}

impl<T: Widget> WidgetAsValue for T {
    fn widget_as_value(&self, tags: &HttpTags) -> Result<Value, HttpError> {
        serde_json::to_value(self).map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to convert widget {} to value: {error}", self.get_type()),
                tags.clone(),
            )
        })
    }
}

pub trait WidgetsAsValue {
    fn widgets_as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError>;
}

impl<T: Widget> WidgetsAsValue for Vec<T> {
    fn widgets_as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError> {
        let mut values = vec![];

        for widget in self {
            values.push(widget.widget_as_value(tags)?);
        }

        Ok(values)
    }
}

pub trait Action: Clone + Serialize {}

pub trait ActionAsValue {
    fn action_as_value(&self, tags: &HttpTags) -> Result<Value, HttpError>;
}

impl<T: Action> ActionAsValue for T {
    fn action_as_value(&self, tags: &HttpTags) -> Result<Value, HttpError> {
        serde_json::to_value(self).map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to convert action to value: {error}"),
                tags.clone(),
            )
        })
    }
}

pub trait ActionsAsValue {
    fn actions_as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError>;
}

impl<T: Widget> ActionsAsValue for Vec<T> {
    fn actions_as_values(&self, tags: &HttpTags) -> Result<Vec<Value>, HttpError> {
        let mut values = vec![];

        for widget in self {
            values.push(widget.widget_as_value(tags)?);
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Alignment {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    Clear,
    Src,
    Dst,
    SrcOver,
    SstOver,
    SrcIn,
    SstIn,
    SrcOut,
    DstOut,
    SrcATop,
    Xor,
    Plus,
    Modulate,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Multiply,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlurStyle {
    Normal,
    Solid,
    Outer,
    Inner,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BoxShape {
    Circle,
    Rectangle,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorderStyle {
    Solid,
    None,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BorderSide {
    StrokeAlignInside,
    StrokeAlignCenter,
    StrokeAlignOutside,
}

impl BorderSide {
    pub fn get_value(&self) -> f64 {
        match self {
            BorderSide::StrokeAlignInside => -1.0,
            BorderSide::StrokeAlignCenter => 0.0,
            BorderSide::StrokeAlignOutside => 1.0,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterQuality {
    None,
    Low,
    Medium,
    High,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageRepeat {
    Repeat,
    RepeatX,
    RepeatY,
    NoRepeat,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BoxFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
    None,
    FitWidth,
    FitHeight,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DecorationImageType {
    File,
    Network,
    Asset,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RectType {
    FromCenter,
    FromCircle,
    FromLTRB,
    FromLTWH,
    FromPoints,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GradientType {
    Linear,
    Radial,
    Sweep,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TileMode {
    Clamp,
    Repeat,
    Mirror,
    Decal,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialColor {
    Primary,
    Shade50,
    Shade100,
    Shade200,
    Shade300,
    Shade400,
    Shade500,
    Shade600,
    Shade700,
    Shade800,
    Shade900,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AlignmentDirectional {
    TopStart,
    TopCenter,
    TopEnd,
    CenterStart,
    Center,
    CenterEnd,
    BottomStart,
    BottomCenter,
    BottomEnd,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OverflowBarAlignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Brightness {
    Dark,
    Light,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BottomNavigationBarType {
    Fixed,
    Shifting,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BottomNavigationBarLandscapeLayout {
    Spread,
    Centered,
    Linear,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialTapTargetSize {
    Padded,
    ShrinkWrap,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FlexFit {
    Tight,
    Loose,
}