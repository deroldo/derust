use crate::envx::Environment;
use crate::httpx::HttpTags;
use axum::http::{Method, Uri};
use lazy_static::lazy_static;
use metrics::Label;
use regex::Regex;

lazy_static! {
    static ref REGEX_MIDDLE: Regex =
        Regex::new("(/[a-fA-F0-9-]{36}/)|(/\\d+/)").expect("Failed to compile regex middle");
    static ref REGEX_END: Regex =
        Regex::new("(/[a-fA-F0-9-]{36})|(/\\d+$|/\\d+\\?)").expect("Failed to compile regex end");
    static ref REGEXES_REPLACE: Vec<(Regex, String)> = vec![
        (REGEX_MIDDLE.clone(), "/{path_param}/".to_string()),
        (REGEX_END.clone(), "/{path_param}".to_string()),
    ];
}

#[derive(Clone, Debug)]
pub struct MetricTag(String, String);

#[derive(Clone, Default, Debug)]
pub struct MetricTags(pub Vec<MetricTag>);

impl MetricTags {
    pub fn vec(&self) -> Vec<MetricTag> {
        self.0.clone()
    }

    pub fn to_labels(
        &self,
        app_name: &str,
        env: &Environment,
        denied_metric_tags: &[String],
        denied_metric_tags_by_regex: &[Regex],
    ) -> Vec<Label> {
        let mut tags = self.clone();
        tags = tags.push("app_name".to_string(), app_name.to_string());
        tags = tags.push("env".to_string(), env.get_name());

        tags.vec()
            .iter()
            .filter(|mt| !denied_metric_tags.contains(&mt.key()))
            .filter(|mt| !denied_metric_tags_by_regex.iter().any(|regex| regex.is_match(&mt.key())))
            .map(|mt| Label::new(mt.key(), mt.value()))
            .collect::<Vec<_>>()
    }

    pub fn push(mut self, key: String, value: String) -> Self {
        self.0.push(MetricTag(key, value));
        self
    }

    pub fn htt_server(req_url: &Uri, req_method: &Method) -> MetricTags {
        let path = normalize_path(req_url.path(), REGEXES_REPLACE.clone());
        MetricTags::from([("method", req_method.as_str()), ("path", &path)])
    }

    pub fn http_client(req_url: &str, req_path: &str, req_method: &str) -> MetricTags {
        let path = normalize_path(req_path, REGEXES_REPLACE.clone());

        MetricTags::from([("method", req_method), ("path", &path), ("url", &req_url)])
    }
}

fn normalize_path(req_path: &str, regexes: Vec<(Regex, String)>) -> String {
    let mut normalized_path = req_path.to_string();

    for (regex, replace) in regexes {
        let replaced_path = regex.replace_all(&normalized_path, replace).to_string();

        normalized_path = replaced_path
            .split("?")
            .next()
            .filter(|it| !it.is_empty() && it.to_string() != "/")
            .unwrap_or("<no_path>")
            .to_string();
    }

    normalized_path
}

impl MetricTag {
    pub fn key(&self) -> String {
        self.0.clone()
    }

    pub fn value(&self) -> String {
        self.1.clone()
    }
}

impl From<&HttpTags> for MetricTags {
    fn from(value: &HttpTags) -> Self {
        let mut tags = MetricTags::default();
        for (key, value) in value.values() {
            tags = tags.push(key, value);
        }
        tags
    }
}

impl From<HttpTags> for MetricTags {
    fn from(value: HttpTags) -> Self {
        let mut tags = MetricTags::default();
        for (key, value) in value.values() {
            tags = tags.push(key, value);
        }
        tags
    }
}

impl<const N: usize> From<[(String, String); N]> for MetricTags {
    fn from(arr: [(String, String); N]) -> Self {
        let mut vec = Vec::with_capacity(N);
        for (k, v) in arr {
            vec.push(MetricTag(k, v));
        }
        Self(vec)
    }
}

impl<const N: usize> From<[(&str, String); N]> for MetricTags {
    fn from(arr: [(&str, String); N]) -> Self {
        let mut vec = Vec::with_capacity(N);
        for (k, v) in arr {
            vec.push(MetricTag(k.to_string(), v));
        }
        Self(vec)
    }
}

impl<const N: usize> From<[(&str, &str); N]> for MetricTags {
    fn from(arr: [(&str, &str); N]) -> Self {
        let mut vec = Vec::with_capacity(N);
        for (k, v) in arr {
            vec.push(MetricTag(k.to_string(), v.to_string()));
        }
        Self(vec)
    }
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
mod test {
    use crate::envx::Environment;
    use crate::metricx::tags::{normalize_path, REGEXES_REPLACE};
    use crate::metricx::MetricTags;
    use regex::Regex;

    #[test]
    fn should_normalize_path() {
        let paths = vec![
            "/anything",
            "/anything/",
            "/anything/123",
            "/anything/123-value",
            "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            "/anything/123-value/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e?foo=bar",
        ];

        let expected_paths = vec![
            "/anything",
            "/anything/",
            "/anything/{path_param}",
            "/anything/123-value",
            "/anything/{path_param}/{path_param}",
            "/anything/123-value/{path_param}",
            "/anything/{path_param}/{path_param}",
        ];

        for (i, path) in paths.iter().enumerate() {
            let normalized_path = normalize_path(path, REGEXES_REPLACE.clone());
            let expected_path = expected_paths[i];
            assert_eq!(expected_path, normalized_path);
        }
    }

    #[tokio::test]
    async fn should_filter_metric_tags() -> Result<(), Box<dyn std::error::Error>> {
        let tags = MetricTags::from([
            ("foo", "bar"),
            ("customer", "1"),
            ("any_id", "2"),
        ]);

        let app_name = "test";
        let env = Environment::Test;
        let denied_tag = "customer".to_string();
        let regex = Regex::new(".+_id$").unwrap();

        let labels = tags.to_labels(
            app_name,
            &env,
            &vec![denied_tag],
            &vec![regex],
        );

        assert_eq!(labels.len(), 3);

        assert_eq!(labels[0].key(), "foo");
        assert_eq!(labels[0].value(), "bar");

        assert_eq!(labels[1].key(), "app_name");
        assert_eq!(labels[1].value(), app_name);

        assert_eq!(labels[2].key(), "env");
        assert_eq!(labels[2].value(), env.get_name());

        Ok(())
    }

    #[derive(Clone)]
    struct AppState;
}
