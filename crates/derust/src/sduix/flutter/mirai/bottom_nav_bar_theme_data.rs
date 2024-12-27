use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::icon_theme_data::IconThemeData;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{BottomNavigationBarLandscapeLayout, BottomNavigationBarType};
use crate::sduix::Color;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BottomNavBarThemeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_icon_theme: Option<IconThemeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unselected_icon_theme: Option<IconThemeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_item_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unselected_item_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_label_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unselected_label_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_selected_labels: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_unselected_labels: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    data_type: Option<BottomNavigationBarType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    landscape_layout: Option<BottomNavigationBarLandscapeLayout>,
}

impl BottomNavBarThemeData {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            background_color: None,
            elevation: None,
            selected_icon_theme: None,
            unselected_icon_theme: None,
            selected_item_color: None,
            unselected_item_color: None,
            selected_label_style: None,
            unselected_label_style: None,
            show_selected_labels: None,
            show_unselected_labels: None,
            data_type: None,
            enable_feedback: None,
            landscape_layout: None,
        }
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_selected_icon_theme(mut self, selected_icon_theme: IconThemeData) -> Self {
        self.selected_icon_theme = Some(selected_icon_theme);
        self
    }

    pub fn with_unselected_icon_theme(mut self, unselected_icon_theme: IconThemeData) -> Self {
        self.unselected_icon_theme = Some(unselected_icon_theme);
        self
    }

    pub fn with_selected_item_color(mut self, selected_item_color: Color) -> Self {
        self.selected_item_color = Some(selected_item_color.hex);
        self
    }

    pub fn with_unselected_item_color(mut self, unselected_item_color: Color) -> Self {
        self.unselected_item_color = Some(unselected_item_color.hex);
        self
    }

    pub fn with_selected_label_style(mut self, selected_label_style: TextStyle) -> Self {
        self.selected_label_style = Some(selected_label_style);
        self
    }

    pub fn with_unselected_label_style(mut self, unselected_label_style: TextStyle) -> Self {
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    pub fn with_show_selected_labels(mut self, show_selected_labels: bool) -> Self {
        self.show_selected_labels = Some(show_selected_labels);
        self
    }

    pub fn with_show_unselected_labels(mut self, show_unselected_labels: bool) -> Self {
        self.show_unselected_labels = Some(show_unselected_labels);
        self
    }

    pub fn with_data_type(mut self, data_type: BottomNavigationBarType) -> Self {
        self.data_type = Some(data_type);
        self
    }

    pub fn with_enable_feedback(mut self, enable_feedback: bool) -> Self {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    pub fn with_landscape_layout(mut self, landscape_layout: BottomNavigationBarLandscapeLayout) -> Self {
        self.landscape_layout = Some(landscape_layout);
        self
    }
}