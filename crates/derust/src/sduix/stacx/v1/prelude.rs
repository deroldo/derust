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
pub use super::state::{
    BinOp, BindKey, Binding, Data, Envelope, Expr, GlobalEntry, GlobalStateBlock, Ident,
    IntoReactive, OnInit, PathSegment, Plain, Reactive, Root, StateBlock, StatePath, UnOp,
};
pub use super::theme::*;
pub use super::widgets::*;
pub use crate::sduix::stacx::common::{
    CustomAction, CustomNode, CustomWidget, Dyn, Path, StateEntry, StateRef, Validate,
    ValidationIssue,
};
