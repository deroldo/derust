use crate::commonx::DerustTags;
use tracing::{debug, error, info, trace, warn};

pub fn trace(message: &str, tags: &DerustTags) {
    trace!(tags = ?tags.values(), message);
}

pub fn debug(message: &str, tags: &DerustTags) {
    debug!(tags = ?tags.values(), message);
}

pub fn info(message: &str, tags: &DerustTags) {
    info!(tags = ?tags.values(), message);
}

pub fn warn(message: &str, tags: &DerustTags) {
    warn!(tags = ?tags.values(), message);
}

pub fn error(message: &str, tags: &DerustTags) {
    error!(tags = ?tags.values(), message);
}
