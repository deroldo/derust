use serde::Serialize;
use crate::httpx::AppContext;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            width,
            height,
        }
    }
}