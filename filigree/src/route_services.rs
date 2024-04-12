use std::{convert::Infallible, net::SocketAddr, path::Path};

use axum::{
    body::Body,
    extract::{ConnectInfo, FromRequestParts, Request},
    response::IntoResponse,
};
use futures::future::BoxFuture;
use http::{HeaderValue, Uri};
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use tower::{Service, ServiceExt};
use tower_http::services::{fs::ServeFileSystemResponseBody, ServeDir};
use tracing::{event, Level};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

/// Forward all requests to another host
#[derive(Clone)]
pub struct ForwardRequest {
    to_host: String,
    client: Client,
}

impl ForwardRequest {
    /// Create a new [ForwardRequest] that forwards requests to `to_host`, without changing the
    /// path
    pub fn new(to_host: String) -> Self {
        Self {
            to_host,
            client: hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new()),
        }
    }
}

impl Service<Request> for ForwardRequest {
    type Response = hyper::Response<Body>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let path = req.uri().path();
        let path_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or(path);

        let url = format!("{}{}", self.to_host, path_query);

        *req.uri_mut() = Uri::try_from(url).unwrap();

        let client = self.client.clone();
        let fut = async move {
            let (mut parts, body) = req.into_parts();

            // Reverse proxies should set x-forwarded-for to the IP address that sent the request.
            // But if it's already set then that means we're already behind another reverse proxy,
            // and so should not overwrite the value that it set.
            if !parts.headers.contains_key("x-forwarded-for") {
                let addr = ConnectInfo::<SocketAddr>::from_request_parts(&mut parts, &())
                    .await
                    .ok()
                    .and_then(|addr| addr.to_string().parse().ok());

                if let Some(addr) = addr {
                    parts.headers.insert("x-forwarded-for", addr);
                }
            }

            let req = Request::from_parts(parts, body);

            let r = client.request(req).await;
            let response = match r {
                Ok(r) => r.into_response(),
                Err(e) => {
                    event!(Level::ERROR, error=?e, "Unable to proxy request");
                    hyper::StatusCode::BAD_GATEWAY.into_response()
                }
            };

            Ok(response)
        };

        Box::pin(fut)
    }
}

/// Serve a directory and return a cache-control header with every successful response
#[derive(Clone, Debug)]
pub struct ServeDirWithCache {
    inner: tower_http::services::ServeDir,
    cache_header: HeaderValue,
}

impl ServeDirWithCache {
    /// Create a new [ServeDirWithCache] with this header value
    pub fn new(dir: tower_http::services::ServeDir, cache_header: HeaderValue) -> Self {
        Self {
            inner: dir,
            cache_header,
        }
    }

    /// Serve a directory with cache header for immutable values. This generally should be used for
    /// static files with hashes in the filename.
    pub fn immutable(dir: tower_http::services::ServeDir) -> Self {
        Self::new(
            dir,
            HeaderValue::from_static("max-age=300, s-maxage=2592000"),
        )
    }
}

impl Service<Request> for ServeDirWithCache {
    type Response = hyper::Response<ServeFileSystemResponseBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        <ServeDir as Service<Request>>::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let clone = self.inner.clone();
        let inner = std::mem::replace(&mut self.inner, clone);
        let header = self.cache_header.clone();
        let fut = async move {
            let mut response = inner.oneshot(req).await?;

            if response.status().is_success() {
                response.headers_mut().insert("cache-control", header);
            }
            Ok(response)
        };

        Box::pin(fut)
    }
}

/// Create a router that serves the _app/immutable directory with appropriate cache headers.
/// The `base_dir` parameter should be the base directory of the built app,
/// which contains the _app/immutable directory.
pub fn serve_immutable_files<P, T>(base_dir: P) -> axum::Router<T>
where
    P: AsRef<std::ffi::OsStr>,
    T: Send + Sync + Clone + 'static,
{
    let full_path = Path::new(base_dir.as_ref()).join("_app/immutable");
    let service = ServeDirWithCache::immutable(
        ServeDir::new(full_path)
            .precompressed_br()
            .precompressed_gzip(),
    );

    axum::Router::new().nest_service("/_app/immutable", service)
}
