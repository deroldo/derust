//! Reactive state vocabulary of the Stac fork (`derust-stac`, branch
//! `estado_compartilhado_reativo`): typed state paths, the expression language and bindings.
//!
//! * [`Ident`] — identifier of the expression language (`[A-Za-z_][A-Za-z0-9_]*`);
//! * [`StatePath`] — a path into the state (`user.name`, `global.token`, `items[0]`); it has no
//!   operators, so a `$state` binding with an operation cannot be built;
//! * [`Expr`] — closed, typed AST of the expression language, printed with the minimum number
//!   of parentheses (an unknown function or a wrong arity cannot be represented);
//! * [`Binding`] — the `{"$state": ".."}` / `{"$expr": ".."}` JSON forms.
//!
//! Everything here is hand written (not generated) and lives under the `sdui_stac_1` feature.

mod binding;
mod expr;
mod ident;
mod path;

pub use binding::Binding;
pub use expr::{BinOp, Expr, UnOp};
pub use ident::Ident;
pub use path::{PathSegment, Root, StatePath};
