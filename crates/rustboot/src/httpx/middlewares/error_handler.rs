use tower_http::catch_panic::{CatchPanicLayer, DefaultResponseForPanic};

pub fn panic_catcher() -> CatchPanicLayer<DefaultResponseForPanic> {
    CatchPanicLayer::new()
}
