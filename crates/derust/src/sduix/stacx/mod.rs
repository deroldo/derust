//! Typed model of the [Stac](https://github.com/StacDev/stac) JSON vocabulary.
//!
//! One module per Stac major (`v1`, ...), each behind its own `sdui_stac_<major>` feature.

pub mod common;

#[cfg(feature = "sdui_stac_1")]
pub mod v1;

/// Returns `(module, stac_version)` for every Stac major enabled at compile time.
pub fn versions() -> &'static [(&'static str, &'static str)] {
    &[
        #[cfg(feature = "sdui_stac_1")]
        ("v1", v1::STAC_VERSION),
    ]
}

#[cfg(all(test, feature = "sdui_stac_1"))]
mod tests {
    use super::*;

    #[test]
    fn versions_lists_enabled_majors() {
        assert_eq!(versions(), &[("v1", "1.6.0")]);
    }
}
