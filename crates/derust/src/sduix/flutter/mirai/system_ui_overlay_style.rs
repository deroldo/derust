use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::widget::Brightness;
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemUiOverlayStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_navigation_bar_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_navigation_bar_divider_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_navigation_bar_icon_brightness: Option<Brightness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_navigation_bar_contrast_enforced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_bar_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_bar_brightness: Option<Brightness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_bar_icon_brightness: Option<Brightness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_status_bar_contrast_enforced: Option<bool>,
}

impl SystemUiOverlayStyle {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            system_navigation_bar_color: None,
            system_navigation_bar_divider_color: None,
            system_navigation_bar_icon_brightness: None,
            system_navigation_bar_contrast_enforced: None,
            status_bar_color: None,
            status_bar_brightness: None,
            status_bar_icon_brightness: None,
            system_status_bar_contrast_enforced: None,
        }
    }

    pub fn with_system_navigation_bar_color(mut self, system_navigation_bar_color: Color) -> Self {
        self.system_navigation_bar_color = Some(system_navigation_bar_color.hex);
        self
    }

    pub fn with_system_navigation_bar_divider_color(mut self, system_navigation_bar_divider_color: Color) -> Self {
        self.system_navigation_bar_divider_color = Some(system_navigation_bar_divider_color.hex);
        self
    }

    pub fn with_system_navigation_bar_icon_brightness(mut self, system_navigation_bar_icon_brightness: Brightness) -> Self {
        self.system_navigation_bar_icon_brightness = Some(system_navigation_bar_icon_brightness);
        self
    }

    pub fn with_system_navigation_bar_contrast_enforced(mut self, system_navigation_bar_contrast_enforced: bool) -> Self {
        self.system_navigation_bar_contrast_enforced = Some(system_navigation_bar_contrast_enforced);
        self
    }

    pub fn with_status_bar_color(mut self, status_bar_color: Color) -> Self {
        self.status_bar_color = Some(status_bar_color.hex);
        self
    }

    pub fn with_status_bar_brightness(mut self, status_bar_brightness: Brightness) -> Self {
        self.status_bar_brightness = Some(status_bar_brightness);
        self
    }

    pub fn with_status_bar_icon_brightness(mut self, status_bar_icon_brightness: Brightness) -> Self {
        self.status_bar_icon_brightness = Some(status_bar_icon_brightness);
        self
    }

    pub fn with_system_status_bar_contrast_enforced(mut self, system_status_bar_contrast_enforced: bool) -> Self {
        self.system_status_bar_contrast_enforced = Some(system_status_bar_contrast_enforced);
        self
    }
}