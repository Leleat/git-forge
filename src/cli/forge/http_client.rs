use reqwest::blocking::RequestBuilder;

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
                    "Auth is enabled but there is a problem with the {env_var} environment variable: {e}"
                )
            }
        };

        Ok(self.header("Authorization", format!("{auth_scheme} {token}")))
    }
}
