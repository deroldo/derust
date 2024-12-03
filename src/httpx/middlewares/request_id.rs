use axum::http::{HeaderName, Request};
use lazy_static::lazy_static;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use uuid::Uuid;

const REQUEST_ID_HEADER_NAME: &str = "x-request-id";

lazy_static! {
    static ref DEFAULT_REQUEST_ID: HeaderName = HeaderName::from_static(REQUEST_ID_HEADER_NAME);
}

#[derive(Clone)]
pub struct UuidRequestId;

impl MakeRequestId for UuidRequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        Uuid::now_v7().to_string().parse().ok().map(RequestId::new)
    }
}

pub fn set_request_id() -> SetRequestIdLayer<UuidRequestId> {
    SetRequestIdLayer::new(DEFAULT_REQUEST_ID.clone(), UuidRequestId)
}

pub fn propagate_request_id() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(DEFAULT_REQUEST_ID.clone())
}
