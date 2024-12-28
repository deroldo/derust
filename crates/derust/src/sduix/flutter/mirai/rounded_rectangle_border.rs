use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::border_radius::BorderRadius;
use crate::sduix::flutter::mirai::border_side::BorderSide;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundedRectangleBorder {
    #[serde(skip_serializing_if = "Option::is_none")]
    side: Option<BorderSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_radius: Option<BorderRadius>,
}

impl RoundedRectangleBorder {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            side: None,
            border_radius: None,
        }
    }

    pub fn with_side(mut self, side: BorderSide) -> Self {
        self.side = Some(side);
        self
    }

    pub fn with_border_radius(mut self, border_radius: BorderRadius) -> Self {
        self.border_radius = Some(border_radius);
        self
    }
}