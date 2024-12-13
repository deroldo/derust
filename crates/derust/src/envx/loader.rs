use crate::envx::Environment;
use dotenv::{dotenv, from_filename};
use serde_json::Value;
use std::collections::HashMap;

#[allow(unused_imports)]
use config::{AsyncSource, Config, ConfigError, Map, ValueKind};

#[cfg(feature = "env_from_secrets_manager")]
use crate::awsx::{load_aws_config, secrets_manager};
#[cfg(feature = "env_from_secrets_manager")]
use aws_sdk_secretsmanager::Client;
use serde::Deserialize;

pub async fn load_app_config<T: for<'a> Deserialize<'a>>(
    environment: Environment,
    prefix: Option<&str>,
    #[cfg(feature = "env_from_secrets_manager")] secrets_manager_ids: Vec<&str>,
) -> Result<T, Box<dyn std::error::Error>> {
    if environment.is_local() || environment.is_test() {
        dotenv().ok();
        from_filename(format!(".env.{}", environment.get_name())).ok();
    }

    let env_source = if let Some(prefix) = prefix {
        config::Environment::with_prefix(prefix).prefix_separator("__")
    } else {
        config::Environment::default()
    }
    .separator("__");

    let builder = Config::builder().add_source(env_source);

    #[cfg(feature = "env_from_secrets_manager")]
    let result = {
        let aws_config = load_aws_config(environment).await;
        let sm_client = secrets_manager(&aws_config);

        let async_source = SecretsManagerSource {
            client: sm_client.client.clone(),
            ids: secrets_manager_ids
                .iter()
                .map(|id| id.to_string())
                .collect(),
            prefix: prefix.map(|prefix| prefix.to_string()),
        };

        builder
            .add_async_source(async_source)
            .build()
            .await?
            .try_deserialize::<T>()
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
    };

    #[cfg(not(feature = "env_from_secrets_manager"))]
    let result = builder
        .build()?
        .try_deserialize::<T>()
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>);

    result
}

#[derive(Debug)]
#[cfg(feature = "env_from_secrets_manager")]
struct SecretsManagerSource {
    client: Client,
    ids: Vec<String>,
    prefix: Option<String>,
}

#[async_trait::async_trait]
#[cfg(feature = "env_from_secrets_manager")]
impl AsyncSource for SecretsManagerSource {
    async fn collect(&self) -> Result<Map<String, config::Value>, ConfigError> {
        let mut map: Map<String, config::Value> = Map::new();

        for id in &self.ids {
            let json = self
                .client
                .get_secret_value()
                .secret_id(id)
                .send()
                .await
                .map_err(|error| {
                    ConfigError::Message(format!(
                        "Failed to load secret for id={id} with error: {error}"
                    ))
                })?
                .secret_string
                .ok_or(ConfigError::Message(format!(
                    "Secret string not found for id={id}"
                )))?;

            let serde_value = restructure_json(&json, &self.prefix).unwrap();

            for (key, value) in convert_serde_value_to_config_map(&serde_value) {
                map.insert(key, value);
            }
        }

        Ok(map)
    }
}

#[allow(dead_code)]
fn restructure_json(input: &str, prefix: &Option<String>) -> Result<Value, serde_json::Error> {
    let parsed: Value = serde_json::from_str(input)?;

    let mut result = serde_json::Map::new();

    if let Some(object) = parsed.as_object() {
        for (key, value) in object {
            let normalized_key = if let Some(prefix) = prefix {
                if !key.starts_with(format!("{}__", prefix).as_str()) {
                    continue;
                }
                key.replace(&format!("{}__", prefix), "")
            } else {
                key.clone()
            };

            let parts: Vec<&str> = normalized_key.split("__").collect();

            let mut current_map = &mut result;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    current_map.insert(part.to_string(), value.clone());
                } else {
                    let next_map = current_map
                        .entry(part.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    current_map = next_map.as_object_mut().unwrap();
                }
            }
        }
    }

    Ok(Value::Object(result))
}

#[allow(dead_code)]
fn convert_serde_value_to_config_map(serde_value: &Value) -> HashMap<String, config::Value> {
    let mut key_map = HashMap::new();

    if let Some(object) = serde_value.as_object() {
        for (key, value) in object {
            let config_value = parse_serde_value(value);
            key_map.insert(key.clone(), config_value);
        }
    }

    key_map
}

#[allow(dead_code)]
fn parse_serde_value(value: &Value) -> config::Value {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                config::Value::new(None, ValueKind::I64(i))
            } else if let Some(f) = n.as_f64() {
                config::Value::new(None, ValueKind::Float(f))
            } else {
                config::Value::new(None, ValueKind::String(n.to_string()))
            }
        }
        Value::Bool(b) => config::Value::new(None, ValueKind::Boolean(*b)),
        Value::Object(inner_object) => {
            let mut inner_map = HashMap::new();
            for (inner_key, inner_value) in inner_object {
                if inner_value.is_object() {
                    let inner_value_converted = convert_serde_value_to_config_map(inner_value);
                    inner_map.insert(
                        inner_key.clone(),
                        config::Value::new(None, Some(ValueKind::Table(inner_value_converted))),
                    );
                } else {
                    inner_map.insert(inner_key.clone(), parse_serde_value(inner_value));
                }
            }
            config::Value::new(None, Some(ValueKind::Table(inner_map)))
        }
        _ => {
            let value_string = if value.is_string() {
                value.as_str().unwrap().to_lowercase()
            } else {
                value.to_string()
            };

            if let Ok(num) = value_string.parse::<u8>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<u16>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<u32>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<u64>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<u128>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<i8>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<i16>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<i32>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<i64>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(num) = value_string.parse::<i128>() {
                config::Value::new(None, ValueKind::from(num))
            } else if let Ok(b) = value_string.parse::<bool>() {
                config::Value::new(None, ValueKind::from(b))
            } else if let Ok(num) = value_string.parse::<f64>() {
                config::Value::new(None, ValueKind::from(num))
            } else {
                config::Value::new(None, ValueKind::from(value_string))
            }
        }
    }
}
