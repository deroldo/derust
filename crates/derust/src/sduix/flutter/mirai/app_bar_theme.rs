use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::icon_theme_data::IconThemeData;
use crate::sduix::flutter::mirai::system_ui_overlay_style::SystemUiOverlayStyle;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::Widget;
use crate::sduix::Color;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBarTheme {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    foreground_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrolled_under_elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_theme: Option<IconThemeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_icon_theme: Option<IconThemeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center_title: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toolbar_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toolbar_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_overlay_style: Option<SystemUiOverlayStyle>,
}

impl Widget for AppBarTheme {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl AppBarTheme {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "appBarTheme".to_string(),
            id: Uuid::now_v7().to_string(),
            background_color: None,
            foreground_color: None,
            elevation: None,
            scrolled_under_elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            actions_icon_theme: None,
            center_title: None,
            title_spacing: None,
            toolbar_height: None,
            toolbar_text_style: None,
            title_text_style: None,
            system_overlay_style: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: Color) -> Self {
        self.foreground_color = Some(foreground_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> Self {
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    pub fn with_shadow_color(mut self, shadow_color: Color) -> Self {
        self.shadow_color = Some(shadow_color.hex);
        self
    }

    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> Self {
        self.surface_tint_color = Some(surface_tint_color.hex);
        self
    }

    pub fn with_icon_theme(mut self, icon_theme: IconThemeData) -> Self {
        self.icon_theme = Some(icon_theme);
        self
    }

    pub fn with_actions_icon_theme(mut self, actions_icon_theme: IconThemeData) -> Self {
        self.actions_icon_theme = Some(actions_icon_theme);
        self
    }

    pub fn with_center_title(mut self, center_title: bool) -> Self {
        self.center_title = Some(center_title);
        self
    }

    pub fn with_title_spacing(mut self, title_spacing: f64) -> Self {
        self.title_spacing = Some(title_spacing);
        self
    }

    pub fn with_toolbar_height(mut self, toolbar_height: f64) -> Self {
        self.toolbar_height = Some(toolbar_height);
        self
    }

    pub fn with_toolbar_text_style(mut self, toolbar_text_style: TextStyle) -> Self {
        self.toolbar_text_style = Some(toolbar_text_style);
        self
    }

    pub fn with_title_text_style(mut self, title_text_style: TextStyle) -> Self {
        self.title_text_style = Some(title_text_style);
        self
    }

    pub fn with_system_overlay_style(mut self, system_overlay_style: SystemUiOverlayStyle) -> Self {
        self.system_overlay_style = Some(system_overlay_style);
        self
    }
}