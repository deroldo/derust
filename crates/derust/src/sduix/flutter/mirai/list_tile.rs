use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{EdgeInsets, WidgetAsValue, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTile {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_tap: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_long_press: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    leading: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trailing: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_three_line: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dense: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<ListTileStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tile_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_tile_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_feedback: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizontal_title_gap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_vertical_padding: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_leading_width: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListTileStyle {
    List,
    Drawer,
}

impl Widget for ListTile {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl ListTile {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "listTile".to_string(),
            id: Uuid::now_v7().to_string(),
            on_tap: None,
            on_long_press: None,
            leading: None,
            title: None,
            subtitle: None,
            trailing: None,
            is_three_line: None,
            dense: None,
            style: None,
            selected_color: None,
            icon_color: None,
            text_color: None,
            content_padding: None,
            enabled: None,
            selected: None,
            focus_color: None,
            hover_color: None,
            autofocus: None,
            tile_color: None,
            selected_tile_color: None,
            enable_feedback: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_on_tap(mut self, on_tap: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_tap = Some(on_tap.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_on_long_press(mut self, on_long_press: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.on_tap = Some(on_long_press.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_leading(mut self, leading: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.leading = Some(leading.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_title(mut self, title: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.title = Some(title.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_subtitle(mut self, subtitle: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.subtitle = Some(subtitle.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_trailing(mut self, trailing: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.trailing = Some(trailing.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_is_three_line(mut self, is_three_line: bool) -> Self {
        self.is_three_line = Some(is_three_line);
        self
    }

    pub fn with_dense(mut self, dense: bool) -> Self {
        self.dense = Some(dense);
        self
    }

    pub fn with_style(mut self, style: ListTileStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_selected_color(mut self, selected_color: Color) -> Self {
        self.selected_color = Some(selected_color.hex);
        self
    }

    pub fn with_icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = Some(icon_color.hex);
        self
    }

    pub fn with_text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color.hex);
        self
    }

    pub fn with_content_padding(mut self, content_padding: EdgeInsets) -> Self {
        self.content_padding = Some(content_padding);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn with_focus_color(mut self, focus_color: Color) -> Self {
        self.focus_color = Some(focus_color.hex);
        self
    }

    pub fn with_hover_color(mut self, hover_color: Color) -> Self {
        self.hover_color = Some(hover_color.hex);
        self
    }

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_tile_color(mut self, tile_color: Color) -> Self {
        self.tile_color = Some(tile_color.hex);
        self
    }

    pub fn with_selected_tile_color(mut self, selected_tile_color: Color) -> Self {
        self.selected_tile_color = Some(selected_tile_color.hex);
        self
    }

    pub fn with_enable_feedback(mut self, enable_feedback: bool) -> Self {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    pub fn with_horizontal_title_gap(mut self, horizontal_title_gap: f64) -> Self {
        self.horizontal_title_gap = Some(horizontal_title_gap);
        self
    }

    pub fn with_min_vertical_padding(mut self, min_vertical_padding: f64) -> Self {
        self.min_vertical_padding = Some(min_vertical_padding);
        self
    }

    pub fn with_min_leading_width(mut self, min_leading_width: f64) -> Self {
        self.min_leading_width = Some(min_leading_width);
        self
    }
}
