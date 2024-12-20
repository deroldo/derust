use serde::Serialize;
use crate::httpx::AppContext;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offset {
    dx: f64,
    dy: f64,
}

impl Offset {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        dx: f64,
        dy: f64,
    ) -> Self {
        Self { dx, dy }
    }
}