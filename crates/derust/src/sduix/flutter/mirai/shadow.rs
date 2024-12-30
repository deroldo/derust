use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::offset::Offset;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shadow {
    color: String,
    offset: Offset,
    blur_radius: f64,
}

impl Shadow {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        color: Color,
        offset: Offset,
        blur_radius: f64,
    ) -> Self {
        Self {
            color: color.hex,
            offset,
            blur_radius,
        }
    }
}
