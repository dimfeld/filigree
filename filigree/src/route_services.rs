use std::{convert::Infallible, net::SocketAddr};

use axum::{
    body::Body,
    extract::{ConnectInfo, FromRequestParts, Request},
    response::IntoResponse,
};
use futures::future::BoxFuture;
use http::Uri;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use tower::Service;
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
