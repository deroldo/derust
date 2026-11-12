//! Stac `v1` vocabulary (targets Stac `1.6.0`).

/// Stac release this module targets.
pub const STAC_VERSION: &str = "1.6.0";

/// Commit of the Stac git tag `v1.6.0` this module was built against.
pub const STAC_TAG_COMMIT: &str = "b6cfe7cdb2bb4432e09d91ba23d6d72bdf721c01";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_pinned_tag() {
        assert_eq!(STAC_VERSION, "1.6.0");
        assert_eq!(STAC_TAG_COMMIT.len(), 40);
    }
}
