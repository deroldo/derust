use crate::envx::EnvironmentError;
use serde::{Deserialize, Serialize};
use std::env;

const ENV_VAR: &str = "ENVIRONMENT";
const LOCAL: &str = "local";
const STAGING: &str = "staging";
const PRODUCTION: &str = "production";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Local,
    Staging,
    Production,
}

impl Environment {
    pub fn detect() -> Result<Environment, EnvironmentError> {
        match env::var(ENV_VAR).ok() {
            Some(env) => Environment::try_from(env),
            None => Err(EnvironmentError::EnvNotFound(ENV_VAR.to_string())),
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Environment::Local)
    }

    pub fn is_staging(&self) -> bool {
        matches!(self, Environment::Staging)
    }

    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    pub fn is_deployed(&self) -> bool {
        matches!(self, Environment::Staging | Environment::Production)
    }
}

impl<'a> TryFrom<&'a str> for Environment {
    type Error = EnvironmentError;

    fn try_from(env: &'a str) -> Result<Self, Self::Error> {
        from_string(env)
    }
}

impl TryFrom<String> for Environment {
    type Error = EnvironmentError;

    fn try_from(env: String) -> Result<Self, Self::Error> {
        from_string(env)
    }
}

fn from_string(env: impl AsRef<str>) -> Result<Environment, EnvironmentError> {
    match env.as_ref().to_lowercase().as_str() {
        LOCAL => Ok(Environment::Local),
        STAGING => Ok(Environment::Staging),
        PRODUCTION => Ok(Environment::Production),
        _ => Err(EnvironmentError::UnknownEnvironment(
            env.as_ref().to_string(),
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_string() {
        assert!(matches!(
            Environment::try_from("local"),
            Ok(Environment::Local)
        ));
        assert!(matches!(
            Environment::try_from("staging"),
            Ok(Environment::Staging)
        ));
        assert!(matches!(
            Environment::try_from("production"),
            Ok(Environment::Production)
        ));

        let unknown = String::from("unknown");

        assert!(
            matches!(Environment::try_from(unknown.clone()), Err(EnvironmentError::UnknownEnvironment(value)) if value == unknown)
        );
    }

    #[test]
    fn get_true_for_specific_environments() {
        let test_data = [
            (Environment::Local, true, false, false, false),
            (Environment::Staging, false, true, false, true),
            (Environment::Production, false, false, true, true),
        ];

        for (env, local, staging, production, deployed) in test_data.iter() {
            assert_eq!(env.is_local(), *local);
            assert_eq!(env.is_staging(), *staging);
            assert_eq!(env.is_production(), *production);
            assert_eq!(env.is_deployed(), *deployed);
        }
    }

    #[test]
    fn detect_environment_from_env_var() {
        let test_data = [
            (LOCAL, Environment::Local),
            (STAGING, Environment::Staging),
            (PRODUCTION, Environment::Production),
        ];

        for (env, expected) in test_data.iter() {
            env::remove_var(ENV_VAR);
            env::set_var(ENV_VAR, env);

            let current_env = Environment::detect().expect("failed to detect environment");

            assert_eq!(current_env, *expected);
        }

        env::remove_var(ENV_VAR);
    }
}
