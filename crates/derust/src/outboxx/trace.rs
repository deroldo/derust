use crate::httpx::HttpTags;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use std::collections::HashMap;

pub(crate) const TRACE_ID_HEADER: &str = "x-trace-id";
#[cfg_attr(not(feature = "aws"), allow(dead_code))]
pub(crate) const SQS_SNS_MAX_ATTRIBUTES: usize = 10;
#[cfg_attr(not(feature = "aws"), allow(dead_code))]
const IDEMPOTENT_KEY_HEADER: &str = "x-idempotent-key";

/// Trace id already in `tags` (any key case, non-empty) or else the current span's trace id.
pub(crate) fn resolve_trace_id(tags: &HttpTags) -> Option<String> {
    tags.values()
        .into_iter()
        .find(|(k, v)| k.eq_ignore_ascii_case(TRACE_ID_HEADER) && !v.is_empty())
        .map(|(_, v)| v)
        .or_else(find_current_trace_id)
}

fn add_trace(
    headers: Option<HashMap<String, String>>,
    trace_id: Option<&str>,
    fits: impl Fn(&HashMap<String, String>) -> bool,
) -> Option<HashMap<String, String>> {
    let Some(trace_id) = trace_id else {
        return headers;
    };
    let empty = HashMap::new();
    let current = headers.as_ref().unwrap_or(&empty);
    if current
        .keys()
        .any(|k| k.eq_ignore_ascii_case(TRACE_ID_HEADER))
        || !fits(current)
    {
        return headers;
    }
    let mut map = headers.unwrap_or_default();
    map.insert(TRACE_ID_HEADER.to_string(), trace_id.to_string());
    Some(map)
}

#[allow(dead_code)]
pub(crate) fn with_trace_header(
    headers: Option<HashMap<String, String>>,
    trace_id: Option<&str>,
) -> Option<HashMap<String, String>> {
    add_trace(headers, trace_id, |_| true)
}

#[cfg_attr(not(feature = "aws"), allow(dead_code))]
pub(crate) fn with_trace_header_limited(
    headers: Option<HashMap<String, String>>,
    trace_id: Option<&str>,
    reserved_slots: usize,
    max: usize,
) -> Option<HashMap<String, String>> {
    add_trace(headers, trace_id, |m| {
        let reserved = if m.contains_key(IDEMPOTENT_KEY_HEADER) {
            0
        } else {
            reserved_slots
        };
        m.len() + reserved < max
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn n(count: usize) -> HashMap<String, String> {
        (0..count)
            .map(|i| (format!("h{i}"), "v".to_string()))
            .collect()
    }

    #[test]
    fn caller_trace_header_any_case_is_kept() {
        for key in ["x-trace-id", "X-Trace-Id", "X-TRACE-ID"] {
            let input = h(&[(key, "mine"), ("a", "b")]);
            let out = with_trace_header(Some(input.clone()), Some("new")).unwrap();
            assert_eq!(out, input);
        }
        let input = h(&[("x-trace-id", "")]);
        assert_eq!(
            with_trace_header(Some(input.clone()), Some("new")),
            Some(input)
        );
    }

    #[test]
    fn adds_trace_keeping_others() {
        let out = with_trace_header(Some(h(&[("a", "b")])), Some("t")).unwrap();
        assert_eq!(out, h(&[("a", "b"), ("x-trace-id", "t")]));
    }

    #[test]
    fn none_with_trace_creates_map() {
        assert_eq!(
            with_trace_header(None, Some("t")),
            Some(h(&[("x-trace-id", "t")]))
        );
        assert_eq!(
            with_trace_header(Some(HashMap::new()), Some("t")),
            Some(h(&[("x-trace-id", "t")]))
        );
    }

    #[test]
    fn without_trace_is_unchanged() {
        assert_eq!(with_trace_header(None, None), None);
        let input = h(&[("a", "b")]);
        assert_eq!(with_trace_header(Some(input.clone()), None), Some(input));
    }

    #[test]
    fn limited_respects_slots() {
        for count in [7, 8] {
            let out = with_trace_header_limited(Some(n(count)), Some("t"), 1, 10).unwrap();
            assert_eq!(out.len(), count + 1);
            assert_eq!(out.get("x-trace-id").map(String::as_str), Some("t"));
        }
        for count in [9, 10] {
            let input = n(count);
            let out = with_trace_header_limited(Some(input.clone()), Some("t"), 1, 10);
            assert_eq!(out, Some(input));
        }
    }

    #[test]
    fn limited_user_idempotent_key_frees_slot() {
        let mut input = n(8);
        input.insert("x-idempotent-key".to_string(), "k".to_string());
        let out = with_trace_header_limited(Some(input), Some("t"), 1, 10).unwrap();
        assert!(out.contains_key("x-trace-id"));
        assert_eq!(out.len(), 10);
    }

    #[test]
    fn limited_none_with_trace_and_caller_trace_kept() {
        assert_eq!(
            with_trace_header_limited(None, Some("t"), 1, 10),
            Some(h(&[("x-trace-id", "t")]))
        );
        let mut full = n(10);
        full.remove("h0");
        full.insert("X-Trace-Id".to_string(), "mine".to_string());
        assert_eq!(
            with_trace_header_limited(Some(full.clone()), Some("t"), 1, 10),
            Some(full)
        );
    }

    #[test]
    fn resolve_uses_tags_then_falls_back() {
        let tags = HttpTags::from([("X-Trace-Id", "abc")]);
        assert_eq!(resolve_trace_id(&tags), Some("abc".to_string()));
        assert_eq!(
            resolve_trace_id(&HttpTags::from([("x-trace-id", "")])),
            None
        );
        assert_eq!(resolve_trace_id(&HttpTags::default()), None);
    }
}
