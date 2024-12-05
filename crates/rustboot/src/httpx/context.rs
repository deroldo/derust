use crate::envx::Environment;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AppContext<C: Deserialize<'static> + Clone + Send + Sync + 'static, S> {
    env: Environment,
    config: C,
    state: S,
}

impl<C: Deserialize<'static> + Clone + Send + Sync + 'static, S> AppContext<C, S> {
    pub fn new(env: Environment, config: C, state: S) -> Self {
        Self { env, config, state }
    }

    pub fn env(&self) -> &Environment {
        &self.env
    }

    pub fn config(&self) -> &C {
        &self.config
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}
