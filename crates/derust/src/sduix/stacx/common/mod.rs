//! Code shared by every Stac major (generic and independent of the Stac version).
//!
//! # Findings from the Stac source (`stac_core` / `stac`, tag `v1.6.0`)
//!
//! * Every widget serializes a `type` discriminator, every action an `actionType`
//!   (`StacWidget.type` / `StacAction.actionType`, see `core/stac_widget.dart` and
//!   `core/stac_action.dart`). The registry in `packages/stac` reads exactly these keys
//!   (`stac_service.dart`).
//! * Models are `json_serializable` classes; the JSON key of each field equals the Dart
//!   field name and enums serialize with their Dart value name. Dart emits `null` for
//!   unset fields; this crate omits them instead (the Stac parsers treat both the same).
//! * `double` fields go through `DoubleConverter`, which writes the number as is, so they
//!   are plain JSON numbers here (`f64`). Non finite values cannot be represented in JSON
//!   and are reported by [`Validate`].
//! * State/template references are `{{key}}` placeholders inside JSON strings. They are
//!   substituted by `resolveVariablesInJson` in `packages/stac` (a regex replace over every
//!   string of the JSON tree), so [`Dyn::Ref`] serializes to the string `"{{key}}"`.
//! * `StacEdgeInsets`, `StacBorderRadius` and `StacTextStyle` accept shorthand forms on
//!   *input*; their `toJson` (what is modeled here) is always the object form.
//! * Stac does not enforce structural constraints on children at parse time (parsers only
//!   fail on malformed values), so [`Validate`] checks what is checkable without guessing:
//!   finite numbers, well formed state references and the few `assert`s found in the
//!   Dart models.

mod dynamic;
mod node;
mod validate;

pub use dynamic::{Dyn, StateEntry, StateRef};
pub use node::{NodeKind, StacNode};
pub use validate::{Path, Validate, ValidationIssue};
