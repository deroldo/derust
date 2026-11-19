use super::super::consumer::Message;

/// Where the idempotency key came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeySource {
    /// The configured message attribute.
    Attribute,
    /// The SQS `message_id` (the attribute was missing or blank).
    MessageIdFallback,
}

/// A resolved idempotency key and its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub(crate) key: String,
    pub(crate) source: KeySource,
}

/// Resolves the idempotency key: a non-blank (after `trim`) `key_attribute` wins;
/// otherwise the `message_id`; if neither exists, `None` (the caller must fail closed).
pub(crate) fn resolve_key(msg: &Message, key_attribute: &str) -> Option<Resolved> {
    if let Some(value) = msg
        .message_attribute(key_attribute)
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        return Some(Resolved {
            key: value.to_string(),
            source: KeySource::Attribute,
        });
    }
    msg.message_id().map(|id| Resolved {
        key: id.to_string(),
        source: KeySource::MessageIdFallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ATTR: &str = "x-idempotent-key";

    #[test]
    fn attribute_wins() {
        let m = Message::builder()
            .message_id("m1")
            .message_attribute(ATTR, "  abc  ")
            .build();
        let r = resolve_key(&m, ATTR).unwrap();
        assert_eq!(r.key, "abc");
        assert_eq!(r.source, KeySource::Attribute);
    }

    #[test]
    fn empty_or_blank_attribute_falls_back() {
        for v in ["", "   "] {
            let m = Message::builder()
                .message_id("m1")
                .message_attribute(ATTR, v)
                .build();
            let r = resolve_key(&m, ATTR).unwrap();
            assert_eq!(r.key, "m1");
            assert_eq!(r.source, KeySource::MessageIdFallback);
        }
    }

    #[test]
    fn missing_attribute_falls_back() {
        let m = Message::builder().message_id("m1").build();
        let r = resolve_key(&m, ATTR).unwrap();
        assert_eq!(r.key, "m1");
        assert_eq!(r.source, KeySource::MessageIdFallback);
    }

    #[test]
    fn nothing_is_none() {
        let m = Message::builder().build();
        assert_eq!(resolve_key(&m, ATTR), None);
        let blank = Message::builder().message_attribute(ATTR, " ").build();
        assert_eq!(resolve_key(&blank, ATTR), None);
    }
}
