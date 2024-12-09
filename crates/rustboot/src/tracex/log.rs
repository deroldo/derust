use tracing::{debug, error, info, trace, warn};
use crate::httpx::HttpTags;

pub fn trace(message: &str, tags: &HttpTags) {
    trace!(tags = ?tags.values(), message);
}

pub fn debug(message: &str, tags: &HttpTags) {
    debug!(tags = ?tags.values(), message);
}

pub fn info(message: &str, tags: &HttpTags) {
    info!(tags = ?tags.values(), message);
}

pub fn warn(message: &str, tags: &HttpTags) {
    warn!(tags = ?tags.values(), message);
}

pub fn error(message: &str, tags: &HttpTags) {
    error!(tags = ?tags.values(), message);
}