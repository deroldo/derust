use crate::httpx::AppContext;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorderRadius {
    top_left: f64,
    top_right: f64,
    bottom_left: f64,
    bottom_right: f64,
}

impl BorderRadius {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        top_left: f64,
        top_right: f64,
        bottom_left: f64,
        bottom_right: f64,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }
}
