use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{Clip, EdgeInsets, MainAxisAlignment, WidgetAsValue, WidgetsAsValue, OverflowBarAlignment, VerticalDirection, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertDialog {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_text_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_alignment: Option<MainAxisAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_overflow_alignment: Option<OverflowBarAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_overflow_direction: Option<VerticalDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions_overflow_button_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    button_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inset_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrollable: Option<bool>,
}

impl Widget for AlertDialog {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl AlertDialog {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "alertDialog".to_string(),
            id: Uuid::now_v7().to_string(),
            icon: None,
            icon_padding: None,
            icon_color: None,
            title: None,
            title_padding: None,
            title_text_style: None,
            content: None,
            content_padding: None,
            content_text_style: None,
            actions: None,
            actions_padding: None,
            actions_alignment: None,
            actions_overflow_alignment: None,
            actions_overflow_direction: None,
            actions_overflow_button_spacing: None,
            button_padding: None,
            background_color: None,
            elevation: None,
            semantic_label: None,
            inset_padding: None,
            clip_behavior: None,
            scrollable: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_icon(mut self, icon: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.icon = Some(icon.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_icon_padding(mut self, icon_padding: EdgeInsets) -> Self {
        self.icon_padding = Some(icon_padding);
        self
    }

    pub fn with_icon_color(mut self, icon_color: Color) -> Self {
        self.icon_color = Some(icon_color.hex);
        self
    }

    pub fn with_title(mut self, title: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.title = Some(title.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_title_padding(mut self, title_padding: EdgeInsets) -> Self {
        self.title_padding = Some(title_padding);
        self
    }

    pub fn with_title_text_style(mut self, title_text_style: TextStyle) -> Self {
        self.title_text_style = Some(title_text_style);
        self
    }

    pub fn with_content(mut self, content: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.content = Some(content.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_content_padding(mut self, content_padding: EdgeInsets) -> Self {
        self.content_padding = Some(content_padding);
        self
    }

    pub fn with_content_text_style(mut self, content_text_style: TextStyle) -> Self {
        self.content_text_style = Some(content_text_style);
        self
    }

    pub fn with_actions(mut self, actions: Vec<impl Widget>, tags: &HttpTags) -> Result<Self, HttpError> {
        self.actions = Some(actions.widgets_as_values(tags)?);
        Ok(self)
    }

    pub fn with_actions_alignment(mut self, actions_alignment: MainAxisAlignment) -> Self {
        self.actions_alignment = Some(actions_alignment);
        self
    }

    pub fn with_actions_padding(mut self, actions_padding: EdgeInsets) -> Self {
        self.actions_padding = Some(actions_padding);
        self
    }

    pub fn with_actions_overflow_alignment(mut self, actions_overflow_alignment: OverflowBarAlignment) -> Self {
        self.actions_overflow_alignment = Some(actions_overflow_alignment);
        self
    }

    pub fn with_actions_overflow_direction(mut self, actions_overflow_direction: VerticalDirection) -> Self {
        self.actions_overflow_direction = Some(actions_overflow_direction);
        self
    }

    pub fn with_actions_overflow_button_spacing(mut self, actions_overflow_button_spacing: f64) -> Self {
        self.actions_overflow_button_spacing = Some(actions_overflow_button_spacing);
        self
    }

    pub fn with_button_padding(mut self, button_padding: EdgeInsets) -> Self {
        self.button_padding = Some(button_padding);
        self
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
        self
    }

    pub fn with_semantic_label(mut self, semantic_label: &str) -> Self {
        self.semantic_label = Some(semantic_label.to_string());
        self
    }

    pub fn with_inset_padding(mut self, inset_padding: EdgeInsets) -> Self {
        self.inset_padding = Some(inset_padding);
        self
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    pub fn with_scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = Some(scrollable);
        self
    }
}