use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use super::{ErrorResponseData, ForceObfuscate};

/// Configuration for [ObfuscateErrorLayer]
#[derive(Debug, Clone)]
pub struct ObfuscateErrorLayerSettings {
    /// Enable the middleware
    pub enabled: bool,
    /// Obfucate 403 forbidden errors
    pub obfuscate_forbidden: bool,
    /// Obfucate 401 unauthorized errors
    pub obfuscate_unauthorized: bool,
}

impl Default for ObfuscateErrorLayerSettings {
    /// The default settings for [ObfuscateErrorLayerSettings] will enable the middleware,
    /// and obfuscate 401 Unauthorized errors, but not opbfuscate 403 Forbidden errors.
    fn default() -> Self {
        ObfuscateErrorLayerSettings {
            enabled: true,
            obfuscate_forbidden: false,
            obfuscate_unauthorized: true,
        }
    }
}

/// A layer that obfuscates error details when running in production.
#[derive(Clone)]
pub struct ObfuscateErrorLayer {
    settings: ObfuscateErrorLayerSettings,
}

impl ObfuscateErrorLayer {
    /// Create a new `ObfuscateErrorLayer` with the given settings.
    pub fn new(settings: ObfuscateErrorLayerSettings) -> ObfuscateErrorLayer {
        ObfuscateErrorLayer { settings }
    }
}

impl<S: Service<Request<Body>>> Layer<S> for ObfuscateErrorLayer {
    type Service = ObfuscateError<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ObfuscateError {
            inner,
            settings: self.settings.clone(),
        }
    }
}

/// The middleware that
#[derive(Debug, Clone)]
pub struct ObfuscateError<S> {
    inner: S,
    settings: ObfuscateErrorLayerSettings,
}

impl<S> Service<Request> for ObfuscateError<S>
where
    S: Service<Request> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: IntoResponse + Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let settings = self.settings.clone();
        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?.into_response();
            if !settings.enabled {
                return Ok(res);
            }

            let force_obfuscate = res.extensions().get::<ForceObfuscate>().cloned();
            let status = res.status();

            let message = match (force_obfuscate, status) {
                (Some(ob), _) => Some(ErrorResponseData::new(
                    ob.kind,
                    ob.message,
                    serde_json::Value::Null,
                )),
                (None, StatusCode::INTERNAL_SERVER_ERROR) => Some(ErrorResponseData::new(
                    "internal_error",
                    "Internal error",
                    serde_json::Value::Null,
                )),
                (None, StatusCode::UNAUTHORIZED) => settings.obfuscate_unauthorized.then(|| {
                    ErrorResponseData::new("unauthorized", "Unauthorized", serde_json::Value::Null)
                }),
                (None, StatusCode::FORBIDDEN) => settings.obfuscate_forbidden.then(|| {
                    ErrorResponseData::new("forbidden", "Forbidden", serde_json::Value::Null)
                }),
                _ => None,
            };

            let Some(message) = message else {
                // This is not an error we need to obfuscate
                return Ok(res);
            };

            let new_res = (status, Json(message)).into_response();

            Ok(new_res)
        })
    }
}

#[cfg(test)]
mod test {
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use serde_json::json;
    use tower::ServiceExt;

    use super::{ObfuscateErrorLayer, ObfuscateErrorLayerSettings};
    use crate::errors::ForceObfuscate;

    fn make_app(enabled: bool) -> Router {
        Router::new()
            .route("/200", get(|| async { (StatusCode::OK, "success") }))
            .route(
                "/500",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "error 500 with info") }),
            )
            .route(
                "/401",
                get(|| async { (StatusCode::UNAUTHORIZED, "error 401 with info") }),
            )
            .route(
                "/403",
                get(|| async { (StatusCode::FORBIDDEN, "error 403 with info") }),
            )
            .layer(ObfuscateErrorLayer::new(ObfuscateErrorLayerSettings {
                enabled,
                obfuscate_unauthorized: true,
                obfuscate_forbidden: true,
            }))
    }

    async fn send_req(app: &Router, url: &str) -> (StatusCode, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(url)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1000000)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    fn make_value(body: &str) -> serde_json::Value {
        serde_json::from_str(body).unwrap()
    }

    #[tokio::test]
    async fn test_disabled() {
        let app = make_app(false);

        let (code, body) = send_req(&app, "/200").await;
        assert_eq!(code, 200, "/200 status code");
        assert_eq!(body, "success", "/200 body");

        let (code, body) = send_req(&app, "/401").await;
        assert_eq!(code, 401, "/401 status code");
        assert_eq!(body, "error 401 with info", "/401 body");

        let (code, body) = send_req(&app, "/403").await;
        assert_eq!(code, 403, "/403 status code");
        assert_eq!(body, "error 403 with info", "/403 body");

        let (code, body) = send_req(&app, "/500").await;
        assert_eq!(code, 500, "/500 status code");
        assert_eq!(body, "error 500 with info", "/500 body");
    }

    #[tokio::test]
    async fn test_enabled() {
        let app = make_app(true);

        let (code, body) = send_req(&app, "/200").await;
        assert_eq!(code, 200, "/200 status code");
        assert_eq!(body, "success", "/200 body");

        let (code, body) = send_req(&app, "/401").await;
        assert_eq!(code, 401, "/401 status code");
        assert_eq!(
            make_value(&body),
            json!({ "error": { "kind": "unauthorized", "message": "Unauthorized", "details": null }}),
            "/401 body should be obfuscated"
        );

        let (code, body) = send_req(&app, "/403").await;
        assert_eq!(code, 403, "/403 status code");
        assert_eq!(
            make_value(&body),
            json!({ "error": { "kind": "forbidden", "message": "Forbidden", "details": null }}),
            "/403 body should be obfuscated"
        );

        let (code, body) = send_req(&app, "/500").await;
        assert_eq!(code, 500, "/500 status code");
        assert_eq!(
            make_value(&body),
            json!({ "error": { "kind": "internal_error", "message": "Internal error", "details": null }}),
            "/500 body should be obfuscated"
        );
    }

    #[tokio::test]
    async fn with_force_obfuscate() {
        let app = Router::new()
            .route(
                "/401",
                get(|| async {
                    let mut res = (StatusCode::UNAUTHORIZED, "error 401 with info").into_response();
                    res.extensions_mut().insert(ForceObfuscate {
                        kind: "force_401".into(),
                        message: "Forced 401".into(),
                    });
                    res
                }),
            )
            .route(
                "/403",
                get(|| async {
                    let mut res = (StatusCode::FORBIDDEN, "error 403 with info").into_response();
                    res.extensions_mut().insert(ForceObfuscate {
                        kind: "force_403".into(),
                        message: "Forced 403".into(),
                    });
                    res
                }),
            )
            .layer(ObfuscateErrorLayer::new(ObfuscateErrorLayerSettings {
                enabled: true,
                obfuscate_unauthorized: false,
                obfuscate_forbidden: true,
            }));

        let (code, body) = send_req(&app, "/401").await;
        assert_eq!(code, 401, "/401 status code");
        assert_eq!(
            make_value(&body),
            json!({ "error": { "kind": "force_401", "message": "Forced 401", "details": null }}),
            "/401 body should be obfuscated"
        );

        let (code, body) = send_req(&app, "/403").await;
        assert_eq!(code, 403, "/403 status code");
        assert_eq!(
            make_value(&body),
            json!({ "error": { "kind": "force_403", "message": "Forced 403", "details": null }}),
            "/403 body should be obfuscated"
        );
    }
}
