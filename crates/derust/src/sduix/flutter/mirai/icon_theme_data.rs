use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::shadow::Shadow;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconThemeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fill: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grade: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optical_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadows: Option<Vec<Shadow>>,
}

impl IconThemeData {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            size: None,
            fill: None,
            weight: None,
            grade: None,
            optical_size: None,
            color: None,
            opacity: None,
            shadows: None,
        }
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_fill(mut self, fill: f64) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_grade(mut self, grade: f64) -> Self {
        self.grade = Some(grade);
        self
    }

    pub fn with_optical_size(mut self, optical_size: f64) -> Self {
        self.optical_size = Some(optical_size);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn with_shadows(mut self, shadows: Vec<Shadow>) -> Self {
        self.shadows = Some(shadows);
        self
    }
}
