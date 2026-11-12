use super::EdgeInsets;

/// Convenience constructors mirroring the named constructors of Dart's `StacEdgeInsets`.
impl EdgeInsets {
    /// Same inset on the four sides.
    pub fn all(value: impl Into<f64>) -> Self {
        let value = value.into();
        Self::only(value, value, value, value)
    }

    /// Explicit insets (`left`, `top`, `right`, `bottom`).
    pub fn only(
        left: impl Into<f64>,
        top: impl Into<f64>,
        right: impl Into<f64>,
        bottom: impl Into<f64>,
    ) -> Self {
        Self::new().left(left).top(top).right(right).bottom(bottom)
    }

    /// Vertical (`top`/`bottom`) and horizontal (`left`/`right`) insets.
    pub fn symmetric(vertical: impl Into<f64>, horizontal: impl Into<f64>) -> Self {
        let (vertical, horizontal) = (vertical.into(), horizontal.into());
        Self::only(horizontal, vertical, horizontal, vertical)
    }

    /// `left`/`right` only.
    pub fn horizontal(value: impl Into<f64>) -> Self {
        let value = value.into();
        Self::new().left(value).right(value)
    }

    /// `top`/`bottom` only.
    pub fn vertical(value: impl Into<f64>) -> Self {
        let value = value.into();
        Self::new().top(value).bottom(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_constructors_match_dart() {
        let json = |v: EdgeInsets| serde_json::to_value(v).unwrap();
        assert_eq!(
            json(EdgeInsets::all(8)),
            serde_json::json!({"left": 8.0, "top": 8.0, "right": 8.0, "bottom": 8.0})
        );
        assert_eq!(
            json(EdgeInsets::symmetric(1, 2)),
            serde_json::json!({"left": 2.0, "top": 1.0, "right": 2.0, "bottom": 1.0})
        );
        assert_eq!(
            json(EdgeInsets::horizontal(3)),
            serde_json::json!({"left": 3.0, "right": 3.0})
        );
    }
}
