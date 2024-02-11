use async_trait::async_trait;
use reqwest::Response;
use tracing::Level;

#[async_trait]
pub trait InspectResponseError {
    async fn print_error_for_status(self) -> reqwest::Result<Response>;
}

#[async_trait]
impl InspectResponseError for reqwest::Response {
    async fn print_error_for_status(self) -> reqwest::Result<Response> {
        let e = self.error_for_status_ref();
        let Err(e) = e else { return Ok(self) };

        let status = e.status().map(|s| s.as_u16()).unwrap_or(0);
        let url = e.url().map(|u| u.as_str()).unwrap_or("<none>");

        let body = self.text().await.unwrap_or_default();

        tracing::event!(Level::ERROR, %status, %url, %body, "Request failed");

        Err(e)
    }
}
