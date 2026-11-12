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
    append_state, cancel_request, decrement_state, increment_state, none_action,
    remove_from_state_at, remove_from_state_value, reset_state, set_state, toggle_item,
    toggle_state, AppendState, BinOp, BindKey, Binding, CancelRequest, Data, DecrementState,
    Envelope, Expr, GlobalEntry, GlobalStateBlock, Ident, IncrementState, IntoReactive, NoneAction,
    OnInit, PathSegment, Plain, Reactive, RemoveFromState, ResetState, Root, SetState, StateBlock,
    StatePath, ToggleItem, ToggleState, UnOp,
};
pub use super::theme::*;
pub use super::widgets::*;
pub use crate::sduix::stacx::common::{
    CustomAction, CustomNode, CustomWidget, Dyn, Path, StateEntry, StateRef, Validate,
    ValidationIssue,
};
