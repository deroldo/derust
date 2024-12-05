#[derive(Debug, thiserror::Error)]
pub enum EnvironmentError {
    #[error("Unknown environment: {0}")]
    UnknownEnvironment(String),
    #[error("Environment variable not found: {0}")]
    EnvNotFound(String),
}
