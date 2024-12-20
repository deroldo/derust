use serde::Serialize;
use crate::httpx::AppContext;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxConstraints {
    min_width: f64,
    max_width: f64,
    min_height: f64,
    max_height: f64,
}

impl BoxConstraints {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        min_width: f64,
        max_width: f64,
        min_height: f64,
        max_height: f64,
    ) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }
}