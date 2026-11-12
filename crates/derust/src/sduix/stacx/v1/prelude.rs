//! Everything needed to build a screen: all types of the major plus the DSL shortcuts.
//!
//! ```
//! use derust::sduix::stacx::v1::prelude::*;
//!
//! let screen = Widget::from(column().children([Widget::from(text("Hello"))]));
//! assert!(screen.validate().is_empty());
//! ```

pub use super::actions::*;
pub use super::dsl::*;
pub use super::foundation::*;
pub use super::theme::*;
pub use super::widgets::*;
pub use crate::sduix::stacx::common::{Dyn, StateEntry, StateRef, Validate, ValidationIssue};
