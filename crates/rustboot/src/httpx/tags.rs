use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Tags(HashMap<String, String>);

impl Tags {
    pub fn ok() -> Self {
        Self(HashMap::new())
    }

    pub fn error(error: Box<dyn std::error::Error>) -> Self {
        Self(HashMap::from([("message".to_string(), error.to_string())]))
    }

    pub fn error_message(message: &str) -> Self {
        Self(HashMap::from([(
            "message".to_string(),
            message.to_string(),
        )]))
    }

    pub fn insert(&mut self, k: &str, v: &str) {
        self.0.insert(k.to_string(), v.to_string());
    }

    pub fn values(&self) -> HashMap<String, String> {
        self.0.clone()
    }
}

impl<const N: usize> From<[(String, String); N]> for Tags {
    fn from(arr: [(String, String); N]) -> Self {
        let mut map = HashMap::with_capacity(N);
        for (k, v) in arr {
            map.insert(k, v);
        }
        Self(map)
    }
}

impl<const N: usize> From<[(&str, &str); N]> for Tags {
    fn from(arr: [(&str, &str); N]) -> Self {
        let mut map = HashMap::with_capacity(N);
        for (k, v) in arr {
            map.insert(k.to_string(), v.to_string());
        }
        Self(map)
    }
}
