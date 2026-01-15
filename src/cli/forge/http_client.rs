use anyhow::Context;
use reqwest::blocking::{RequestBuilder, Response};
use serde::de::DeserializeOwned;

const USER_AGENT: &str = "git-forge";

pub struct HttpClient {
    reqwest_client: reqwest::blocking::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            reqwest_client: reqwest::blocking::Client::new(),
        }
    }

    pub fn get(&self, url: &str) -> RequestBuilder {
        self.reqwest_client
            .get(url)
            .header("User-Agent", USER_AGENT)
    }

    pub fn post(&self, url: &str) -> RequestBuilder {
        self.reqwest_client
            .post(url)
            .header("User-Agent", USER_AGENT)
    }
}

/// A paginated response from a forge API.
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub has_next_page: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, has_next_page: bool) -> Self {
        Self {
            items,
            has_next_page,
        }
    }

    /// Transforms the items while preserving pagination metadata.
    pub fn map<U, F>(self, f: F) -> PaginatedResponse<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResponse {
            items: self.items.into_iter().map(f).collect(),
            has_next_page: self.has_next_page,
        }
    }

    /// Filters and transforms items while preserving pagination metadata.
    pub fn filter_map<U, F>(self, f: F) -> PaginatedResponse<U>
    where
        F: FnMut(T) -> Option<U>,
    {
        PaginatedResponse {
            items: self.items.into_iter().filter_map(f).collect(),
            has_next_page: self.has_next_page,
        }
    }
}

impl<T> TryFrom<Response> for PaginatedResponse<T>
where
    T: DeserializeOwned,
{
    type Error = anyhow::Error;

    fn try_from(value: Response) -> Result<Self, Self::Error> {
        // Forges use Link headers for pagination
        let has_next_page = value
            .headers()
            .get("link")
            .and_then(|value| value.to_str().ok())
            .map(|link_header| {
                link_header
                    .split(',')
                    .any(|link| link.contains("rel=\"next\""))
            })
            .unwrap_or(false);

        let items = value
            .json::<Vec<T>>()
            .context("Failed to parse API response")?;

        Ok(PaginatedResponse::new(items, has_next_page))
    }
}

pub trait WithAuth {
    fn with_auth(
        self,
        use_auth: bool,
        env_var: &str,
        auth_scheme: &str,
    ) -> anyhow::Result<RequestBuilder>;
}

impl WithAuth for RequestBuilder {
    fn with_auth(
        self,
        use_auth: bool,
        env_var: &str,
        auth_scheme: &str,
    ) -> anyhow::Result<RequestBuilder> {
        if !use_auth {
            return Ok(self);
        }

        let token = match std::env::var(env_var) {
            Ok(token) => token,
            Err(e) => {
                anyhow::bail!(
                    "There is a problem with the environment variable ({env_var}) used for authentication: {e}"
                )
            }
        };

        Ok(self.header("Authorization", format!("{auth_scheme} {token}")))
    }
}

pub trait WithHttpStatusOk {
    /// Middleware to make sure that we have a 200 status.
    fn with_http_status_ok(self) -> anyhow::Result<Response>;
}

impl WithHttpStatusOk for Response {
    fn with_http_status_ok(self) -> anyhow::Result<Response> {
        let url = self.url().to_string();
        let status = self.status();

        if !status.is_success() {
            let error_body = self
                .text()
                .unwrap_or_else(|_| String::from("(unable to read response body)"));

            anyhow::bail!(
                "HTTP {status}\n\
                 URL: {url}\n\
                 Response: {error_body}",
            );
        }

        Ok(self)
    }
}
