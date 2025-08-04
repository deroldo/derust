use crate::httpx::{AppContext, HttpError, HttpTags};
use opentelemetry::trace::TraceContextExt;
use reqwest::{Client, Method, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_tracing::TracingMiddleware;
use serde::Deserialize;
use std::time::Duration;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags, Stopwatch};

#[derive(Clone)]
pub struct HttpClient {
    client: ClientWithMiddleware,
    base_url: String,
}

pub struct Response<T: for<'de> Deserialize<'de>> {
    pub status_code: StatusCode,
    pub body: Option<T>,
}

impl HttpClient {
    pub async fn new(
        user_agent: &str,
        base_url: &str,
        timeout: u64,
        connection_timeout: u64,
    ) -> Result<HttpClient, Box<dyn std::error::Error>> {
        let reqwest_client = Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_millis(timeout))
            .connect_timeout(Duration::from_millis(connection_timeout))
            .build()?;

        let client = ClientBuilder::new(reqwest_client)
            .with(TracingMiddleware::default())
            .build();

        Ok(HttpClient {
            client: client,
            base_url: base_url.to_string(),
        })
    }

    pub async fn get<'a, T, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        S: Clone,
    {
        let req = self
            .client
            .get(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::GET, &self.base_url, path);
        send(context, request_context, req, None::<&()>, headers, None, tags).await
    }

    pub async fn post<'a, T, B, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        body: &B,
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        B: serde::Serialize,
        S: Clone,
    {
        let req = self
            .client
            .post(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::POST, &self.base_url, path);
        send(context, request_context, req, Some(body), headers, None, tags).await
    }

    pub async fn form<'a, T, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        form: &[(&str, &str)],
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        S: Clone,
    {
        let req = self
            .client
            .post(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::POST, &self.base_url, path);
        send(context, request_context, req, None::<serde_json::Value>.as_ref(), headers, Some(form), tags).await
    }

    pub async fn put<'a, T, B, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        body: &B,
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        B: serde::Serialize,
        S: Clone,
    {
        let req = self
            .client
            .post(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::PUT, &self.base_url, path);
        send(context, request_context, req, Some(body), headers, None, tags).await
    }

    pub async fn patch<'a, T, B, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        body: &B,
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        B: serde::Serialize,
        S: Clone,
    {
        let req = self
            .client
            .post(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::PATCH, &self.base_url, path);
        send(context, request_context, req, Some(body), headers, None, tags).await
    }

    pub async fn delete<'a, T, B, S>(
        &self,
        context: &AppContext<S>,
        path: &str,
        query_params: Option<Vec<(&str, &str)>>,
        headers: Option<Vec<(&str, &str)>>,
        tags: &HttpTags,
    ) -> Result<Response<T>, HttpError>
    where
        T: for<'de> Deserialize<'de>,
        B: serde::Serialize,
        S: Clone,
    {
        let req = self
            .client
            .post(full_url(&self.base_url, path, query_params));
        let request_context = RequestContext::new(Method::DELETE, &self.base_url, path);
        send(context, request_context, req, None::<&B>, headers, None, tags).await
    }
}

fn full_url(base_url: &str, url: &str, query_params: Option<Vec<(&str, &str)>>) -> String {
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

    format!("{}{}{}", base_url, url, params)
}

async fn send<'a, T, B, S>(
    context: &AppContext<S>,
    request_context: RequestContext,
    mut request_builder: RequestBuilder,
    body: Option<&B>,
    headers: Option<Vec<(&str, &str)>>,
    form: Option<&[(&str, &str)]>,
    tags: &HttpTags,
) -> Result<Response<T>, HttpError>
where
    T: for<'de> Deserialize<'de>,
    B: serde::Serialize,
    S: Clone,
{
    if let Some(b) = body {
        request_builder = request_builder.json(b);
    }

    if let Some(values) = form {
        request_builder = request_builder.form(values);
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

    let status_code = res.status();

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    stopwatch.record(MetricTags::from([(
        "status",
        status_code.as_u16().to_string(),
    )]));

    if status_code.is_success() && status_code.as_u16() != 204 {
        let body = res.json().await.map_err(|error| {
            HttpError::without_body(
                error.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                format!("Failed to deserialize response: {error}"),
                tags.clone(),
            )
        })?;

        Ok(Response {
            status_code,
            body: Some(body),
        })
    } else if status_code.as_u16() == 204 {
        Ok(Response {
            status_code,
            body: None,
        })
    } else {
        let response_body = res
            .json()
            .await;

        let response = match response_body {
            Ok(body) => HttpError::with_json(
                status_code,
                format!("Http response error: {body}"),
                body,
                tags.clone(),
            ),
            Err(error) => HttpError::without_body(
                status_code,
                format!("Failed to get http response error: {error}"),
                tags.clone(),
            ),
        };

        Err(response)
    }
}

struct RequestContext {
    method: Method,
    url: String,
    path: String,
}

impl RequestContext {
    fn new(method: Method, url: &str, path: &str) -> Self {
        let url_without_protocol = url.replace("https://", "").replace("http://", "");

        let normalized_path = if path.is_empty() || path == "/" {
            "<no_path>"
        } else {
            path.split('?').next().unwrap_or("<no_path>")
        };

        Self {
            method,
            url: url_without_protocol,
            path: normalized_path.to_string(),
        }
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
        let urls_and_paths = vec![
            ("https://www.rust-lang.org", ""),
            ("https://www.rust-lang.org", "/"),
            ("https://www.rust-lang.org", "/anything"),
            ("https://www.rust-lang.org", "/anything/"),
            ("https://www.rust-lang.org", "/anything/123"),
            (
                "https://www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e",
            ),
            (
                "https://www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e?foo=bar",
            ),
            (
                "https://www.rust-lang.org",
                "/anything/123/0193a2ce-e912-762e-a66b-5b45d44a3a6e?foo=bar",
            ),
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

        for (i, (url, path)) in urls_and_paths.iter().enumerate() {
            let rc = RequestContext::new(Method::GET, url, path);
            let (expected_url, expected_path) = expected_urls_and_paths[i];
            assert_eq!(expected_url, rc.url);
            assert_eq!(expected_path, rc.path);
        }
    }
}
