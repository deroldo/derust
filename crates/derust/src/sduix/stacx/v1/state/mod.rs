//! Reactive state vocabulary of the Stac fork (`derust-stac`, branch
//! `estado_compartilhado_reativo`): typed state paths, the expression language and bindings.
//!
//! * [`Ident`] — identifier of the expression language (`[A-Za-z_][A-Za-z0-9_]*`);
//! * [`StatePath`] — a path into the state (`user.name`, `global.token`, `items[0]`); it has no
//!   operators, so a `$state` binding with an operation cannot be built;
//! * [`Expr`] — closed, typed AST of the expression language, printed with the minimum number
//!   of parentheses (an unknown function or a wrong arity cannot be represented);
//! * [`Binding`] — the `{"$state": ".."}` / `{"$expr": ".."}` JSON forms.
//! * [`Reactive`] / [`IntoReactive`] — a property that is a literal or a binding;
//! * [`Data`] — free-form JSON data with bindings;
//! * [`SetState`], [`IncrementState`], [`ToggleItem`], ... — the state actions;
//! * [`ItemTemplate`], [`ItemVars`] — list templates (`items`, `itemTemplate`, `as`, `indexAs`);
//! * [`Envelope`], [`BindKey`], [`StateBlock`], [`GlobalStateBlock`], [`GlobalEntry`], [`OnInit`]
//!   — properties common to every widget and the state declarations.
//!
//! Everything here is hand written (not generated) and lives under the `sdui_stac_1` feature.

mod actions;
mod binding;
mod data;
mod decl;
mod envelope;
mod expr;
mod ident;
mod path;
mod reactive;
mod template;

pub use actions::{
    append_state, cancel_request, decrement_state, increment_state, none_action,
    remove_from_state_at, remove_from_state_value, reset_state, set_state, toggle_item,
    toggle_state, Actions, AppendState, CancelRequest, DecrementState, IncrementState, NoneAction,
    RemoveFromState, ResetState, SetState, ToggleItem, ToggleState,
};
pub use binding::Binding;
pub use data::Data;
pub use decl::{BindKey, GlobalEntry, GlobalStateBlock, OnInit, StateBlock};
pub use envelope::Envelope;
pub use expr::{BinOp, Expr, UnOp};
pub use ident::Ident;
pub use path::{PathSegment, Root, StatePath};
pub use reactive::{IntoReactive, Plain, Reactive};
pub use template::{ItemTemplate, ItemVars, TemplateName};
