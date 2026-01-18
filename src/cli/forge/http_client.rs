use reqwest::blocking::{RequestBuilder, Response};

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
}

pub trait IntoPaginatedResponse<T> {
    fn into_paginated_response(self, has_next_page: bool) -> PaginatedResponse<T>;
}

impl<S, T: From<S>> IntoPaginatedResponse<T> for Vec<S> {
    fn into_paginated_response(self, has_next_page: bool) -> PaginatedResponse<T> {
        PaginatedResponse::new(self.into_iter().map(Into::into).collect(), has_next_page)
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

pub fn has_next_link_header(response: &Response) -> bool {
    response
        .headers()
        .get("link")
        .and_then(|value| value.to_str().ok())
        .map(|link_header| {
            link_header
                .split(',')
                .any(|link| link.contains("rel=\"next\""))
        })
        .unwrap_or(false)
}
