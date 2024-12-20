use serde::Serialize;
use crate::httpx::AppContext;
use crate::sduix::Color;
use crate::sduix::flutter::mirai::alignment_geometry::AlignmentGeometry;
use crate::sduix::flutter::mirai::widget::{Alignment, GradientType, TileMode};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gradient {
    colors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stops: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    begin: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center: Option<Alignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gradient_type: Option<GradientType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focal: Option<AlignmentGeometry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tile_mode: Option<TileMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focal_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_angle: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_angle: Option<f64>,
}

impl Gradient {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        colors: Vec<Color>,
    ) -> Self {
        Self {
            colors: colors.iter().map(|c| c.hex.to_string()).collect(),
            stops: None,
            begin: None,
            end: None,
            center: None,
            gradient_type: None,
            focal: None,
            tile_mode: None,
            focal_radius: None,
            radius: None,
            start_angle: None,
            end_angle: None,
        }
    }

    pub fn with_stops(mut self, stops: Vec<f64>) -> Self {
        self.stops = Some(stops);
        self
    }

    pub fn with_begin(mut self, begin: Alignment) -> Self {
        self.begin = Some(begin);
        self
    }

    pub fn with_end(mut self, end: Alignment) -> Self {
        self.end = Some(end);
        self
    }

    pub fn with_center(mut self, center: Alignment) -> Self {
        self.center = Some(center);
        self
    }

    pub fn with_gradient_type(mut self, gradient_type: GradientType) -> Self {
        self.gradient_type = Some(gradient_type);
        self
    }

    pub fn with_focal(mut self, focal: AlignmentGeometry) -> Self {
        self.focal = Some(focal);
        self
    }

    pub fn with_tile_mode(mut self, tile_mode: TileMode) -> Self {
        self.tile_mode = Some(tile_mode);
        self
    }

    pub fn with_focal_radius(mut self, focal_radius: f64) -> Self {
        self.focal_radius = Some(focal_radius);
        self
    }

    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn with_start_angle(mut self, start_angle: f64) -> Self {
        self.start_angle = Some(start_angle);
        self
    }

    pub fn with_end_angle(mut self, end_angle: f64) -> Self {
        self.end_angle = Some(end_angle);
        self
    }
}