use crate::httpx::AppContext;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentGeometry {
    dx: f64,
    dy: f64,
}

impl AlignmentGeometry {
    pub fn new<S: Clone>(_context: &AppContext<S>, dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }
}
