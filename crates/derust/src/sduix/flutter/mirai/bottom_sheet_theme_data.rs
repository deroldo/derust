use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::border::Border;
use crate::sduix::flutter::mirai::box_constraints::BoxConstraints;
use crate::sduix::flutter::mirai::size::Size;
use crate::sduix::flutter::mirai::widget::Clip;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BottomSheetThemeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modal_background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modal_barrier_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modal_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_drag_handle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drag_handle_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drag_handle_size: Option<Size>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    constraints: Option<BoxConstraints>,
}

impl BottomSheetThemeData {
    pub fn new<S: Clone>(_context: &AppContext<S>) -> Self {
        Self {
            background_color: None,
            surface_tint_color: None,
            elevation: None,
            modal_background_color: None,
            modal_barrier_color: None,
            shadow_color: None,
            modal_elevation: None,
            shape: None,
            show_drag_handle: None,
            drag_handle_color: None,
            drag_handle_size: None,
            clip_behavior: None,
            constraints: None,
        }
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> Self {
        self.surface_tint_color = Some(surface_tint_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_modal_background_color(mut self, modal_background_color: Color) -> Self {
        self.modal_background_color = Some(modal_background_color.hex);
        self
    }

    pub fn with_modal_barrier_color(mut self, modal_barrier_color: Color) -> Self {
        self.modal_barrier_color = Some(modal_barrier_color.hex);
        self
    }

    pub fn with_shadow_color(mut self, shadow_color: Color) -> Self {
        self.shadow_color = Some(shadow_color.hex);
        self
    }

    pub fn with_modal_elevation(mut self, modal_elevation: f64) -> Self {
        self.modal_elevation = Some(modal_elevation);
        self
    }

    pub fn with_shape(mut self, shape: Border) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn with_show_drag_handle(mut self, show_drag_handle: bool) -> Self {
        self.show_drag_handle = Some(show_drag_handle);
        self
    }

    pub fn with_drag_handle_color(mut self, drag_handle_color: Color) -> Self {
        self.drag_handle_color = Some(drag_handle_color.hex);
        self
    }

    pub fn with_drag_handle_size(mut self, drag_handle_size: Size) -> Self {
        self.drag_handle_size = Some(drag_handle_size);
        self
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    pub fn with_constraints(mut self, constraints: BoxConstraints) -> Self {
        self.constraints = Some(constraints);
        self
    }
}
