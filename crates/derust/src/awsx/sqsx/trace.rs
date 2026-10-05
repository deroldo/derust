use super::consumer::Message;
use crate::httpx::HttpTags;
use tracing::warn;

pub(crate) const TRACE_ID_ATTRIBUTE: &str = "x-trace-id";
const MAX_TRACE_ID_LEN: usize = 128;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TraceResolution {
    pub id: String,
    pub generated: bool,
}

fn is_valid(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.len() <= MAX_TRACE_ID_LEN
        && candidate.chars().all(|c| c.is_ascii_graphic())
}

fn new_trace_id() -> String {
    loop {
        let n: u128 = rand::random();
        if n != 0 {
            return format!("{n:032x}");
        }
    }
}

pub(crate) fn resolve(msg: &Message) -> TraceResolution {
    if let Some(raw) = msg.message_attribute(TRACE_ID_ATTRIBUTE) {
        let candidate = raw.trim();
        if is_valid(candidate) {
            return TraceResolution {
                id: candidate.to_string(),
                generated: false,
            };
        }
        warn!(
            "Invalid {} message attribute, generating a new trace id :: message_id={}",
            TRACE_ID_ATTRIBUTE,
            msg.message_id().unwrap_or("unknown")
        );
    }
    TraceResolution {
        id: new_trace_id(),
        generated: true,
    }
}

/// Fresh `HttpTags` (per message) holding `x-trace-id`, plus the id as a `String`.
pub(crate) fn tags_for(msg: &Message) -> (HttpTags, String) {
    let id = resolve(msg).id;
    let mut tags = HttpTags::default();
    tags.add(TRACE_ID_ATTRIBUTE, &id);
    (tags, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(v: Option<&str>) -> Message {
        let mut b = Message::builder().message_id("m");
        if let Some(v) = v {
            b = b.message_attribute(TRACE_ID_ATTRIBUTE, v);
        }
        b.build()
    }

    fn is_32_hex(s: &str) -> bool {
        s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn valid_attribute_is_used() {
        let r = resolve(&msg(Some("abc-123")));
        assert_eq!(r.id, "abc-123");
        assert!(!r.generated);
    }

    #[test]
    fn attribute_is_trimmed() {
        assert_eq!(resolve(&msg(Some("  abc  "))).id, "abc");
    }

    #[test]
    fn missing_generates_32_hex() {
        let r = resolve(&msg(None));
        assert!(r.generated);
        assert!(is_32_hex(&r.id));
        assert_ne!(r.id, "0".repeat(32));
    }

    #[test]
    fn invalid_values_generate_new() {
        let long = "a".repeat(129);
        for v in ["", "   ", long.as_str(), "a b", "a\nb", "ação"] {
            let r = resolve(&msg(Some(v)));
            assert!(r.generated, "value {v:?}");
            assert!(is_32_hex(&r.id), "value {v:?}");
        }
        let ok = "a".repeat(128);
        assert!(!resolve(&msg(Some(&ok))).generated);
    }

    #[test]
    fn generated_ids_differ() {
        assert_ne!(resolve(&msg(None)).id, resolve(&msg(None)).id);
    }

    #[test]
    fn tags_for_holds_trace() {
        let (tags, id) = tags_for(&msg(Some("t1")));
        assert_eq!(id, "t1");
        assert_eq!(
            tags.values().get("x-trace-id").map(String::as_str),
            Some("t1")
        );
    }
}
