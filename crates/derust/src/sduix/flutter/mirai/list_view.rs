use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::sduix::flutter::mirai::widget::{Axis, Clip, DragStartBehavior, EdgeInsets, ScrollPhysics, ScrollViewKeyboardDismissBehavior, Widget, WidgetAsValue, WidgetsAsValue};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListView {
    #[serde(rename = "type")]
    widget_type: String,
    id: String,
    children: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_direction: Option<Axis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reverse: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    physics: Option<ScrollPhysics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shrink_wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<EdgeInsets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    add_automatic_keep_alives: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    add_repaint_boundaries: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    add_semantic_indexes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_extent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_child_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drag_start_behavior: Option<DragStartBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard_dismiss_behavior: Option<ScrollViewKeyboardDismissBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    restoration_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_behavior: Option<Clip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<Value>,
}

impl Widget for ListView {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_type(&self) -> String {
        self.widget_type.clone()
    }
}

impl ListView {
    pub fn new<S: Clone>(
        _context: &AppContext<S>,
        children: Vec<impl Widget>,
        tags: &HttpTags,
    ) -> Result<Self, HttpError> {
        Ok(Self {
            widget_type: "listView".to_string(),
            id: Uuid::now_v7().to_string(),
            children: children.as_values(tags)?,
            scroll_direction: None,
            reverse: None,
            primary: None,
            physics: None,
            shrink_wrap: None,
            padding: None,
            add_automatic_keep_alives: None,
            add_repaint_boundaries: None,
            add_semantic_indexes: None,
            cache_extent: None,
            semantic_child_count: None,
            drag_start_behavior: None,
            keyboard_dismiss_behavior: None,
            restoration_id: None,
            clip_behavior: None,
            separator: None,
        })
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn with_scroll_direction(mut self, scroll_direction: Axis) -> Self {
        self.scroll_direction = Some(scroll_direction);
        self
    }

    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = Some(reverse);
        self
    }

    pub fn with_primary(mut self, primary: bool) -> Self {
        self.primary = Some(primary);
        self
    }

    pub fn with_physics(mut self, physics: ScrollPhysics) -> Self {
        self.physics = Some(physics);
        self
    }

    pub fn with_shrink_wrap(mut self, shrink_wrap: bool) -> Self {
        self.shrink_wrap = Some(shrink_wrap);
        self
    }

    pub fn with_padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn with_add_automatic_keep_alives(mut self, add_automatic_keep_alives: bool) -> Self {
        self.add_automatic_keep_alives = Some(add_automatic_keep_alives);
        self
    }

    pub fn with_add_repaint_boundaries(mut self, add_repaint_boundaries: bool) -> Self {
        self.add_repaint_boundaries = Some(add_repaint_boundaries);
        self
    }

    pub fn with_add_semantic_indexes(mut self, add_semantic_indexes: bool) -> Self {
        self.add_semantic_indexes = Some(add_semantic_indexes);
        self
    }

    pub fn with_cache_extent(mut self, cache_extent: f64) -> Self {
        self.cache_extent = Some(cache_extent);
        self
    }

    pub fn with_semantic_child_count(mut self, semantic_child_count: u64) -> Self {
        self.semantic_child_count = Some(semantic_child_count);
        self
    }

    pub fn with_drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> Self {
        self.drag_start_behavior = Some(drag_start_behavior);
        self
    }

    pub fn with_keyboard_dismiss_behavior(mut self, keyboard_dismiss_behavior: ScrollViewKeyboardDismissBehavior) -> Self {
        self.keyboard_dismiss_behavior = Some(keyboard_dismiss_behavior);
        self
    }

    pub fn with_restoration_id(mut self, restoration_id: &str) -> Self {
        self.restoration_id = Some(restoration_id.to_string());
        self
    }

    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> Self {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    pub fn with_separator(mut self, separator: impl Widget, tags: &HttpTags) -> Result<Self, HttpError> {
        self.separator = Some(separator.as_value(tags)?);
        Ok(self)
    }
}