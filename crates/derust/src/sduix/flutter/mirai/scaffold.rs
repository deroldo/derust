use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::app_bar::AppBar;
use crate::sduix::flutter::mirai::widget::{FloatingActionButtonLocation, WidgetAsValue, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scaffold {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_bar: Option<AppBar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<Value>,
    background_color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    floating_action_button: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom_navigation_bar: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom_sheet: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize_to_avoid_bottom_inset: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extend_body: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extend_body_behind_app_bar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    floating_action_button_location: Option<FloatingActionButtonLocation>,
}

impl Widget for Scaffold {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Scaffold {
    pub fn new<S: Clone>(
        context: &AppContext<S>,
    ) -> Self {
        Self {
            widget_type: "scaffold".to_string(),
            id: Uuid::now_v7().to_string(),
            app_bar: None,
            body: None,
            background_color: context.theme().background_color.hex.clone(),
            floating_action_button: None,
            bottom_navigation_bar: None,
            bottom_sheet: None,
            resize_to_avoid_bottom_inset: None,
            primary: None,
            extend_body: None,
            extend_body_behind_app_bar: None,
            floating_action_button_location: None,
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn with_app_bar(mut self, app_bar: AppBar) -> Self {
        self.app_bar = Some(app_bar);
        self
    }

    pub fn with_body(mut self, body: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.body = Some(body.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_floating_action_button(mut self, floating_action_button: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.floating_action_button = Some(floating_action_button.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_bottom_navigation_bar(mut self, bottom_navigation_bar: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.bottom_navigation_bar = Some(bottom_navigation_bar.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_bottom_sheet(mut self, bottom_sheet: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.bottom_sheet = Some(bottom_sheet.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = background_color.hex;
        self
    }

    pub fn with_resize_to_avoid_bottom_inset(mut self, resize_to_avoid_bottom_inset: bool) -> Self {
        self.resize_to_avoid_bottom_inset = Some(resize_to_avoid_bottom_inset);
        self
    }

    pub fn with_primary(mut self, primary: bool) -> Self {
        self.primary = Some(primary);
        self
    }

    pub fn with_extend_body(mut self, extend_body: bool) -> Self {
        self.extend_body = Some(extend_body);
        self
    }

    pub fn with_extend_body_behind_app_bar(mut self, extend_body_behind_app_bar: bool) -> Self {
        self.extend_body_behind_app_bar = Some(extend_body_behind_app_bar);
        self
    }

    pub fn with_floating_action_button_location(mut self, floating_action_button_location: FloatingActionButtonLocation) -> Self {
        self.floating_action_button_location = Some(floating_action_button_location);
        self
    }
}
