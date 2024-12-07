use crate::httpx::{AppContext, HttpError, HttpTags};
use opentelemetry::trace::TraceContextExt;
use reqwest::{Client, Method, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_tracing::TracingMiddleware;
use std::time::Duration;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags, Stopwatch};

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

struct RequestContext {
    method: Method,
    url: String,
    path: String,
}

impl RequestContext {
    fn new(method: Method, full_url: &str) -> Self {
        let full_url_without_protocol = full_url.replace("https://", "").replace("http://", "");
        let url = full_url_without_protocol
            .split('/')
            .next()
            .unwrap()
            .to_string();
        let full_path = full_url_without_protocol.replace(&url, "");
        let path = full_path
            .split("?")
            .next()
            .filter(|it| !it.is_empty() && it.to_string() != "/")
            .unwrap_or("<no_path>")
            .to_string();
        Self { method, url, path }
    }
}

pub async fn get<'a, T, B, S>(
    context: AppContext<S>,
    client: &ClientWithMiddleware,
    url: &str,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
{
    let req = client.get(full_url(url, query_params));
    let request_context = RequestContext::new(Method::GET, url);
    send(context, request_context, req, None::<&B>, headers, tags).await
}

pub async fn post<'a, T, B, S>(
    context: AppContext<S>,
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
{
    let req = client.post(full_url(url, query_params));
    let request_context = RequestContext::new(Method::POST, url);
    send(context, request_context, req, Some(body), headers, tags).await
}

pub async fn put<'a, T, B, S>(
    context: AppContext<S>,
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
{
    let req = client.put(full_url(url, query_params));
    let request_context = RequestContext::new(Method::PUT, url);
    send(context, request_context, req, Some(body), headers, tags).await
}

pub async fn patch<'a, T, B, S>(
    context: AppContext<S>,
    client: &ClientWithMiddleware,
    url: &str,
    body: &B,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
{
    let req = client.patch(full_url(url, query_params));
    let request_context = RequestContext::new(Method::PATCH, url);
    send(context, request_context, req, Some(body), headers, tags).await
}

pub async fn delete<'a, T, B, S>(
    context: AppContext<S>,
    client: &ClientWithMiddleware,
    url: &str,
    query_params: Option<Vec<(&str, &str)>>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
{
    let req = client.delete(full_url(url, query_params));
    let request_context = RequestContext::new(Method::DELETE, url);
    send(context, request_context, req, None::<&B>, headers, tags).await
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

async fn send<'a, T, B, S>(
    context: AppContext<S>,
    request_context: RequestContext,
    mut request_builder: RequestBuilder,
    body: Option<&B>,
    headers: Option<Vec<(&str, &str)>>,
    tags: HttpTags,
) -> Result<T, HttpError>
where
    T: serde::de::DeserializeOwned,
    B: serde::Serialize,
    S: Clone,
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

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = start_stopwatch(&context, request_context);

    let res = request_builder.send().await.map_err(|error| {
        HttpError::without_body(
            error.status().unwrap_or(StatusCode::BAD_GATEWAY),
            format!("Failed to send http request: {error}"),
            tags.clone(),
        )
    })?;

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    stopwatch.record(MetricTags::from([(
        "status",
        res.status().as_u16().to_string(),
    )]));

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

#[cfg(any(feature = "statsd", feature = "prometheus"))]
fn start_stopwatch<S>(context: &AppContext<S>, req: RequestContext) -> Stopwatch<S>
where
    S: Clone,
{
    let metric_tags = MetricTags::http_client(&req.url, &req.path, req.method.as_str());
    timer::start_stopwatch(&context, "http_client_seconds", metric_tags)
}

#[cfg(feature = "http_client")]
mod test {
    use super::*;
    use crate::http_clientx::client::RequestContext;

    #[test]
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    fn should_remove_params_and_split_path_from_url() {
        let urls = vec![
            "https://www.rust-lang.org",
            "https://www.rust-lang.org/",
            "https://www.rust-lang.org/anything",
            "https://www.rust-lang.org/anything/",
            "https://www.rust-lang.org/anything/123",
            "https://www.rust-lang.org/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            "https://www.rust-lang.org/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e?foo=bar",
            "http://www.rust-lang.org/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e?foo=bar",
        ];

        let expected_urls_and_paths = vec![
            ("www.rust-lang.org", "<no_path>"),
            ("www.rust-lang.org", "<no_path>"),
            ("www.rust-lang.org", "/anything"),
            ("www.rust-lang.org", "/anything/"),
            ("www.rust-lang.org", "/anything/123"),
            (
                "www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            ),
            (
                "www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            ),
            (
                "www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            ),
        ];

        for (i, url) in urls.iter().enumerate() {
            let rc = RequestContext::new(Method::GET, url);
            let (expected_url, expected_path) = expected_urls_and_paths[i];
            assert_eq!(expected_url, rc.url);
            assert_eq!(expected_path, rc.path);
        }
    }
}
