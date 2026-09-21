//! Re-export direto da API pública da crate oficial `growthbook-rust` (versão fixada
//! em `crates/derust/Cargo.toml`). O derust não adiciona nenhuma camada própria de
//! configuração, cliente ou tratamento de erro sobre o GrowthBook — use os tipos e
//! funções nativos da SDK diretamente através deste módulo.

pub use growthbook_rust::client::{GrowthBookClient, GrowthBookClientTrait};
pub use growthbook_rust::error::{GrowthbookError, GrowthbookErrorCode};
pub use growthbook_rust::model_public::{
    Experiment, ExperimentResult, FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue,
};
