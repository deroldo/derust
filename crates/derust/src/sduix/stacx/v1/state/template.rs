use super::data::Data;
use super::expr::Expr;
use super::ident::Ident;
use super::path::StatePath;
use super::reactive::{IntoReactive, Plain, Reactive};
use crate::sduix::stacx::common::{Path, Validate, ValidationIssue};
use serde::{Serialize, Serializer};

const DEFAULT_ITEM: &str = "item";
const DEFAULT_INDEX: &str = "index";

/// Name of the variable a list template binds (`as` / `indexAs`).
///
/// An empty text is accepted by the contract and means "use the default"; it is never written
/// to the JSON by the list widgets. A non-empty text must be a valid [`Ident`] (reported by
/// `Validate`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TemplateName(String);

impl TemplateName {
    /// Wraps `name` without checking it.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The text as given (possibly empty).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the name is empty (the default applies).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn effective(&self, default: &str) -> String {
        if self.0.is_empty() {
            default.to_string()
        } else {
            self.0.clone()
        }
    }
}

impl From<&str> for TemplateName {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TemplateName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Serialize for TemplateName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl Validate for TemplateName {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if !self.0.is_empty() {
            Ident::new(self.0.as_str()).validate_at(path, issues);
        }
    }
}

impl Plain for TemplateName {}

/// The data side of a list template: the entries and the names of the two variables.
///
/// Passed to the `template` method of `ListView`, `GridView`, `PageView`, `SliverList` and
/// `SliverGrid`, together with a closure that receives the [`ItemVars`] for the names chosen
/// here. That is the only way to reference the entry, so the references always use the
/// effective names (`as` / `indexAs`, or `item` / `index` by default).
///
/// # Per-item state
///
/// A template has no state of its own per item. Per-item state is a **set of ids kept in a
/// screen key** (for example `selected`): write it with `toggleItem` / `appendState` /
/// `removeFromState` and read it with [`ItemVars::in_list`] and [`ItemVars::count`]. `bind`
/// does not fit per item: it reads and writes a whole screen key. Editing a field of an entry
/// in place is not covered by the contract.
///
/// ```
/// use derust::sduix::stacx::v1::prelude::*;
/// use derust::sduix::stacx::v1::state::ItemTemplate;
///
/// let todos = StatePath::key("todos");
/// let list = list_view().template(ItemTemplate::new(todos).item_as("todo"), |vars| {
///     Widget::from(text(vars.item_field("name")))
/// });
/// assert!(list.validate().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ItemTemplate {
    items: Reactive<Vec<Data>>,
    item_name: TemplateName,
    index_name: TemplateName,
}

impl ItemTemplate {
    /// Template over `items`: a literal list, a state path or an expression.
    pub fn new(items: impl IntoReactive<Vec<Data>>) -> Self {
        Self {
            items: items.into_reactive(),
            item_name: TemplateName::default(),
            index_name: TemplateName::default(),
        }
    }

    /// Name of the variable that holds the entry (`as`; default `item`; empty = default).
    pub fn item_as(mut self, name: impl Into<TemplateName>) -> Self {
        self.item_name = name.into();
        self
    }

    /// Name of the variable that holds the 0-based position (`indexAs`; default `index`;
    /// empty = default).
    pub fn index_as(mut self, name: impl Into<TemplateName>) -> Self {
        self.index_name = name.into();
        self
    }

    /// The variables of this template, with the effective names.
    pub fn vars(&self) -> ItemVars {
        ItemVars {
            item_name: self.item_name.effective(DEFAULT_ITEM),
            index_name: self.index_name.effective(DEFAULT_INDEX),
        }
    }

    /// `(items, as, indexAs)`; the names are `None` when empty (the key is then not written).
    pub fn into_parts(
        self,
    ) -> (
        Reactive<Vec<Data>>,
        Option<TemplateName>,
        Option<TemplateName>,
    ) {
        let keep = |name: TemplateName| (!name.is_empty()).then_some(name);
        (self.items, keep(self.item_name), keep(self.index_name))
    }
}

/// The variables of a list template, with the **effective** names (`as` / `indexAs`, or the
/// defaults `item` / `index`).
///
/// See [`ItemTemplate`] for how per-item state is modeled (a set of ids in a screen key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemVars {
    item_name: String,
    index_name: String,
}

impl ItemVars {
    /// The entry (`item`, or the `as` name).
    pub fn item(&self) -> StatePath {
        StatePath::key(self.item_name.as_str())
    }

    /// The 0-based position (`index`, or the `indexAs` name).
    pub fn index(&self) -> StatePath {
        StatePath::key(self.index_name.as_str())
    }

    /// A field of the entry: `item_field("id")` is `item.id`.
    pub fn item_field(&self, name: impl Into<Ident>) -> StatePath {
        self.item().field(name)
    }

    /// Whether this entry's id is in the screen key `list`: `contains(list, item.id)`.
    pub fn in_list(&self, list: StatePath) -> Expr {
        Expr::contains(list, self.item_field("id"))
    }

    /// Number of entries in the screen key `list`: `length(list)`.
    pub fn count(&self, list: StatePath) -> Expr {
        Expr::length(list)
    }
}
