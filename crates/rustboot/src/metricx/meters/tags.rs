use crate::httpx::AppContext;
use metrics::Label;

#[derive(Clone, Debug)]
pub struct MetricTag(String, String);

#[derive(Clone, Default, Debug)]
pub struct MetricTags(pub Vec<MetricTag>);

impl MetricTags {
    pub fn vec(&self) -> Vec<MetricTag> {
        self.0.clone()
    }

    pub fn to_labels<S>(&self, context: AppContext<S>) -> Vec<Label>
    where
        S: Clone,
    {
        let mut tags = self.clone();
        tags = tags.push("app_name".to_string(), context.app_name().to_string());
        tags = tags.push("env".to_string(), format!("{:?}", context.env()));

        tags.vec()
            .iter()
            .filter(|mt| !context.denied_metric_tags().contains(&mt.key()))
            .map(|mt| Label::new(mt.key(), mt.value()))
            .collect::<Vec<_>>()
    }

    pub fn push(mut self, key: String, value: String) -> Self {
        self.0.push(MetricTag(key, value));
        self
    }
}

impl MetricTag {
    pub fn key(&self) -> String {
        self.0.clone()
    }

    pub fn value(&self) -> String {
        self.1.clone()
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
