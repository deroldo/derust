use crate::httpx::AppContext;
use crate::sduix::flutter::mirai::widget::Brightness;
use serde::Serialize;
use crate::sduix::Color;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorScheme {
    brightness: Brightness,
    primary: String,
    on_primary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_primary_container: Option<String>,
    secondary: String,
    on_secondary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    secondary_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_secondary_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tertiary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_tertiary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tertiary_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_tertiary_container: Option<String>,
    error: String,
    on_error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_error_container: Option<String>,
    background: String,
    on_background: String,
    surface: String,
    on_surface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_surface_variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outline_variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inverse_surface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_inverse_surface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inverse_primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    surface_tint: Option<String>,
}

impl ColorScheme {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        brightness: Brightness,
        primary: Color,
        on_primary: Color,
        secondary: Color,
        on_secondary: Color,
        error: Color,
        on_error: Color,
        background: Color,
        on_background: Color,
        surface: Color,
        on_surface: Color,
    ) -> Self {
        Self {
            brightness,
            primary: primary.hex,
            on_primary: on_primary.hex,
            primary_container: None,
            on_primary_container: None,
            secondary: secondary.hex,
            on_secondary: on_secondary.hex,
            secondary_container: None,
            on_secondary_container: None,
            tertiary: None,
            on_tertiary: None,
            tertiary_container: None,
            on_tertiary_container: None,
            error: error.hex,
            on_error: on_error.hex,
            error_container: None,
            on_error_container: None,
            background: background.hex,
            on_background: on_background.hex,
            surface: surface.hex,
            on_surface: on_surface.hex,
            surface_variant: None,
            on_surface_variant: None,
            outline: None,
            outline_variant: None,
            shadow: None,
            scrim: None,
            inverse_surface: None,
            on_inverse_surface: None,
            inverse_primary: None,
            surface_tint: None,
        }
    }

    pub fn with_primary_container(mut self, primary_container: Color) -> Self {
        self.primary_container = Some(primary_container.hex);
        self
    }

    pub fn with_on_primary_container(mut self, on_primary_container: Color) -> Self {
        self.on_primary_container = Some(on_primary_container.hex);
        self
    }

    pub fn with_secondary_container(mut self, secondary_container: Color) -> Self {
        self.secondary_container = Some(secondary_container.hex);
        self
    }

    pub fn with_on_secondary_container(mut self, on_secondary_container: Color) -> Self {
        self.on_secondary_container = Some(on_secondary_container.hex);
        self
    }

    pub fn with_tertiary(mut self, tertiary: Color) -> Self {
        self.tertiary = Some(tertiary.hex);
        self
    }

    pub fn with_on_tertiary(mut self, on_tertiary: Color) -> Self {
        self.on_tertiary = Some(on_tertiary.hex);
        self
    }

    pub fn with_tertiary_container(mut self, tertiary_container: Color) -> Self {
        self.tertiary_container = Some(tertiary_container.hex);
        self
    }

    pub fn with_on_tertiary_container(mut self, on_tertiary_container: Color) -> Self {
        self.on_tertiary_container = Some(on_tertiary_container.hex);
        self
    }

    pub fn with_error_container(mut self, error_container: Color) -> Self {
        self.error_container = Some(error_container.hex);
        self
    }

    pub fn with_on_error_container(mut self, on_error_container: Color) -> Self {
        self.on_error_container = Some(on_error_container.hex);
        self
    }

    pub fn with_surface_variant(mut self, surface_variant: Color) -> Self {
        self.surface_variant = Some(surface_variant.hex);
        self
    }

    pub fn with_on_surface_variant(mut self, on_surface_variant: Color) -> Self {
        self.on_surface_variant = Some(on_surface_variant.hex);
        self
    }

    pub fn with_outline(mut self, outline: Color) -> Self {
        self.outline = Some(outline.hex);
        self
    }

    pub fn with_outline_variant(mut self, outline_variant: Color) -> Self {
        self.outline_variant = Some(outline_variant.hex);
        self
    }

    pub fn with_shadow(mut self, shadow: Color) -> Self {
        self.shadow = Some(shadow.hex);
        self
    }

    pub fn with_scrim(mut self, scrim: Color) -> Self {
        self.scrim = Some(scrim.hex);
        self
    }

    pub fn with_inverse_surface(mut self, inverse_surface: Color) -> Self {
        self.inverse_surface = Some(inverse_surface.hex);
        self
    }

    pub fn with_on_inverse_surface(mut self, on_inverse_surface: Color) -> Self {
        self.on_inverse_surface = Some(on_inverse_surface.hex);
        self
    }

    pub fn with_inverse_primary(mut self, inverse_primary: Color) -> Self {
        self.inverse_primary = Some(inverse_primary.hex);
        self
    }

    pub fn with_surface_tint(mut self, surface_tint: Color) -> Self {
        self.surface_tint = Some(surface_tint.hex);
        self
    }
}