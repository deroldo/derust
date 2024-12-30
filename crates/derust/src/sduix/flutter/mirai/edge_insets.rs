use crate::httpx::AppContext;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeInsets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<f64>,
}

impl EdgeInsets {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            top: None,
            left: None,
            bottom: None,
            right: None,
        }
    }

    pub fn with_top(mut self, top: f64) -> Self {
        self.top = Some(top);
        self
    }

    pub fn with_left(mut self, left: f64) -> Self {
        self.left = Some(left);
        self
    }

    pub fn with_bottom(mut self, bottom: f64) -> Self {
        self.bottom = Some(bottom);
        self
    }

    pub fn with_right(mut self, right: f64) -> Self {
        self.right = Some(right);
        self
    }
}
