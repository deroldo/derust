#[derive(Clone)]
pub struct Color {
    pub hex: String,
}

impl Color {
    pub fn new(hex: &str) -> Self {
        Self {
            hex: hex.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Theme {
    pub primary_color: Color,
    pub secondary_color: Color,
    pub background_color: Color,
    pub scaffold_background_color: Color,
    pub text_theme: TextTheme,
}

impl Theme {
    pub fn new(
        primary_color: Color,
        secondary_color: Color,
        background_color: Color,
        scaffold_background_color: Color,
        text_theme: TextTheme,
    ) -> Self {
        Self {
            primary_color,
            secondary_color,
            background_color,
            scaffold_background_color,
            text_theme,
        }
    }
}

#[derive(Clone)]
pub struct TextTheme {
    pub primary_color: Color,
    pub secondary_color: Color,
    pub font_large_size: f64,
    pub font_medium_size: f64,
    pub font_small_size: f64,
}

impl TextTheme {
    pub fn new(
        primary_color: Color,
        secondary_color: Color,
        font_large_size: f64,
        font_medium_size: f64,
        font_small_size: f64,
    ) -> Self {
        Self {
            primary_color,
            secondary_color,
            font_large_size,
            font_medium_size,
            font_small_size,
        }
    }
}
