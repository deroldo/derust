use crate::httpx::{HttpError, Tags};
use opentelemetry::trace::TraceContextExt;
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_tracing::TracingMiddleware;
use std::time::Duration;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn new(
    user_agent: &str,
    timeout: u64,
    connection_timeout: u64,
) -> Result<ClientWithMiddleware, Box<dyn std::error::Error>> {
    let reqwest_client = Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_millis(timeout))
        .connect_timeout(Duration::from_millis(connection_timeout))
        .build()?;

    Ok(ClientBuilder::new(reqwest_client)
        .with(TracingMiddleware::default())
        .build())
}

pub async fn get<'a, T, B>(
    client: &ClientWithMiddleware,
    url: &str,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    let req = client.get(full_url(url, query_params));
    send::<T, B>(req, None, headers, tags).await
}

pub async fn post<'a, T, B>(
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    let req = client.post(full_url(url, query_params));
    send(req, Some(body), headers, tags).await
}

pub async fn put<'a, T, B>(
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    let req = client.put(full_url(url, query_params));
    send(req, Some(body), headers, tags).await
}

pub async fn patch<'a, T, B>(
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    let req = client.patch(full_url(url, query_params));
    send(req, Some(body), headers, tags).await
}

pub async fn delete<'a, T, B>(
    client: &ClientWithMiddleware,
    url: &str,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    let req = client.delete(full_url(url, query_params));
    send::<T, B>(req, None, headers, tags).await
}

fn full_url(url: &str, query_params: Option<Vec<(&str, &str)>>) -> String {
    let params = if let Some(params) = query_params {
        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&");
        format!("?{}", query)
    } else {
        "".to_string()
    };

    format!("{}{}", url, params)
}

async fn send<'a, T, B>(
    mut request_builder: RequestBuilder,
    body: Option<&B>,
    headers: Option<Vec<(&str, &str)>>,
    tags: Tags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
{
    if let Some(b) = body {
        request_builder = request_builder.json(b);
    }

    for (key, value) in headers.unwrap_or_default() {
        request_builder = request_builder.header(key, value);
    }

    if let Some(trace_parent) = get_traceparent() {
        request_builder = request_builder.header("traceparent", &trace_parent);
    }

    let res = request_builder.send().await.map_err(|error| {
        HttpError::without_body(
            error.status().unwrap_or(StatusCode::BAD_GATEWAY),
            format!("Failed to send http request: {error}"),
            tags.clone(),
        )
    })?;

    if res.status().is_success() {
        res.json().await.map_err(|error| {
            HttpError::without_body(
                error.status().unwrap_or(StatusCode::BAD_GATEWAY),
                format!("Failed to deserialize response: {error}"),
                tags.clone(),
            )
        })
    } else {
        let status_code = res.status();
        let response_message = res
            .text()
            .await
            .unwrap_or("Failed to get response body as text".to_string());
        Err(HttpError::without_body(
            status_code,
            format!("Http response error: {response_message}"),
            tags.clone(),
        ))
    }
}

fn get_traceparent() -> Option<String> {
    let ctx = Span::current().context();
    let binding = ctx.span();
    let span_context = binding.span_context();

    if span_context.is_valid() {
        Some(format!(
            "{:02x}-{:032x}-{:016x}-{:02x}",
            span_context.trace_flags().to_u8(),
            span_context.trace_id(),
            span_context.span_id(),
            span_context.trace_flags().to_u8()
        ))
    } else {
        None
    }
}
