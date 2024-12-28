use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::border_side::BorderSide;
use crate::sduix::flutter::mirai::icon_theme_data::IconThemeData;
use crate::sduix::flutter::mirai::rounded_rectangle_border::RoundedRectangleBorder;
use crate::sduix::flutter::mirai::text_style::TextStyle;
use crate::sduix::flutter::mirai::widget::{EdgeInsets, MaterialTapTargetSize, WidgetAsValue, Widget};
use crate::sduix::Color;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chip {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    label: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_style: Option<TextStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_icon: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_icon_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_button_tooltip_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    side: Option<BorderSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<RoundedRectangleBorder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autofocus: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elevation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_theme: Option<IconThemeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material_tap_target_size: Option<MaterialTapTargetSize>,
}

impl Widget for Chip {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl Chip {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        widget: impl Widget,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "chip".to_string(),
            id: Uuid::now_v7().to_string(),
            label: widget.widget_as_value(tags)?,
            avatar: None,
            label_style: None,
            label_padding: None,
            delete_icon: None,
            delete_icon_color: None,
            delete_button_tooltip_message: None,
            side: None,
            shape: None,
            autofocus: None,
            color: None,
            background_color: None,
            padding: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            material_tap_target_size: None,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_avatar(mut self, avatar: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.avatar = Some(avatar.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_label_style(mut self, label_style: TextStyle) -> Self {
        self.label_style = Some(label_style);
        self
    }

    pub fn with_label_padding(mut self, label_padding: EdgeInsets) -> Self {
        self.label_padding = Some(label_padding);
        self
    }

    pub fn with_delete_icon(mut self, delete_icon: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.delete_icon = Some(delete_icon.widget_as_value(tags)?);
        Ok(self)
    }

    pub fn with_delete_icon_color(mut self, delete_icon_color: Color) -> Self {
        self.delete_icon_color = Some(delete_icon_color.hex);
        self
    }

    pub fn with_delete_button_tooltip_message(mut self, delete_button_tooltip_message: &str) -> Self {
        self.delete_button_tooltip_message = Some(delete_button_tooltip_message.to_string());
        self
    }

    pub fn with_side(mut self, side: BorderSide) -> Self {
        self.side = Some(side);
        self
    }

    pub fn with_shape(mut self, shape: RoundedRectangleBorder) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn with_autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = Some(autofocus);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color.hex);
        self
    }

    pub fn with_background_color(mut self, background_color: Color) -> Self {
        self.background_color = Some(background_color.hex);
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = Some(elevation);
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

    pub fn with_material_tap_target_size(mut self, material_tap_target_size: MaterialTapTargetSize) -> Self {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }
}