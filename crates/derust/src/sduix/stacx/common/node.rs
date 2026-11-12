/// Whether a node is a widget or an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// Serialized with a `type` discriminator.
    Widget,
    /// Serialized with an `actionType` discriminator.
    Action,
}

impl NodeKind {
    /// JSON key holding the discriminator for this kind of node.
    pub const fn discriminator_key(self) -> &'static str {
        match self {
            NodeKind::Widget => "type",
            NodeKind::Action => "actionType",
        }
    }
}

/// Implemented by every widget/action struct: describes how it is identified in JSON.
pub trait StacNode {
    /// Whether the node is a widget or an action.
    const KIND: NodeKind;
    /// Value of the discriminator (`type` / `actionType`).
    const TYPE_NAME: &'static str;

    /// Value of the discriminator (`type` / `actionType`).
    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }
}
