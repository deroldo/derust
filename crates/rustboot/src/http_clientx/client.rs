use std::time::Duration;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::{find_current_context, find_current_trace_id};
use reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, RequestBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use crate::httpx::{HttpError, Tags};

pub async fn new(
    timeout: u64,
    connection_timeout: u64,
) -> Result<ClientWithMiddleware, Box<dyn std::error::Error>> {
    let reqwest_client = Client::builder()
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
        let query = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<String>>().join("&");
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

    if let Some(trace_id) = find_current_trace_id() {
        request_builder = request_builder.header("traceId", &trace_id);
        request_builder = request_builder.header("x-trace-id", &trace_id);
        request_builder = request_builder.header("x-b3-trace-id", &trace_id);
    }

    let res = request_builder.send().await.map_err(|error| HttpError::without_body(
        error.status().unwrap_or(StatusCode::BAD_GATEWAY),
        format!("Failed to send http request: {error}"),
        tags.clone(),
    ))?;

    if res.status().is_success() {
        res.json().await.map_err(|error| HttpError::without_body(
            error.status().unwrap_or(StatusCode::BAD_GATEWAY),
            format!("Failed to deserialize response: {error}"),
            tags.clone(),
        ))
    } else {
        let status_code = res.status();
        let response_message = res.text().await.unwrap_or("Failed to get response body as text".to_string());
        Err(HttpError::without_body(
            status_code,
            format!("Http response error: {response_message}"),
            tags.clone(),
        ))
    }
}