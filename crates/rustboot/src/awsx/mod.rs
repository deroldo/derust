mod sns;
mod sqs;

pub use sns::*;
pub use sqs::*;

use aws_config::{BehaviorVersion, SdkConfig};

pub async fn load_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}