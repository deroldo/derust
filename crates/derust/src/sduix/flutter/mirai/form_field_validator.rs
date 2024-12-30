use crate::httpx::AppContext;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFieldValidator {
    rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl FormFieldValidator {
    pub fn new<S: Clone>(_context: &AppContext<S>, rule: &str) -> Self {
        Self {
            rule: rule.to_string(),
            message: None,
        }
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }
}
