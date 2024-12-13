use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 9011;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    port: u16,
}

impl Config {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { port: DEFAULT_PORT }
    }
}
