use anyhow::Context;

use crate::{
    cli::{
        forge::{
            ForgeClient,
            http_client::{HttpClient, WithAuth},
        },
        issue::{Issue, IssueState},
        pr::{Pr, PrState},
        web::WebTarget,
    },
    git::GitRemoteData,
};

// =============================================================================
// Domain Types
// =============================================================================

const AUTH_TOKEN: &str = "GITEA_TOKEN";
const AUTH_SCHEME: &str = "token";

pub struct GiteaClient {
    api_url: Option<String>,
    remote: Option<GitRemoteData>,
    http_client: HttpClient,
}

impl GiteaClient {
    pub fn new(remote: Option<GitRemoteData>, api_url: Option<String>) -> Self {
        Self {
            remote,
            http_client: HttpClient::new(),
            api_url,
        }
    }

    fn get_api_base_url(&self) -> anyhow::Result<String> {
        if let Some(api_url) = &self.api_url {
            return Ok(api_url.clone());
        }

        let (host, port) = match self.remote.as_ref() {
            Some(v) => (&v.host, v.port),
            None => anyhow::bail!("No remote data available and no API URL provided"),
        };
        let base_url = match port {
            Some(p) => format!("https://{host}:{p}/api/v1"),
            None => format!("https://{host}/api/v1"),
        };

        Ok(base_url)
    }
}

impl ForgeClient for GiteaClient {
    fn get_issues(
        &self,
        use_auth: bool,
        author: Option<&str>,
        labels: &[String],
        page: u32,
        per_page: u32,
        state: IssueState,
    ) -> anyhow::Result<Vec<Issue>> {
        let base_url = self.get_api_base_url()?;
        let repo_path = match self.remote.as_ref() {
            Some(v) => &v.path,
            None => anyhow::bail!("No remote data available"),
        };
        let url = format!("{base_url}/repos/{repo_path}/issues");
        let mut request = self
            .http_client
            .get(&url)
            .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
            .query(&[("state", state)])
            .query(&[("page", page)])
            .query(&[("limit", per_page)])
            .query(&[("type", "issues")]);

        if let Some(author) = author {
            request = request.query(&[("created_by", author)]);
        }

        if !labels.is_empty() {
            request = request.query(&[("labels", labels.join(","))]);
        }

        let issues = request
            .send()
            .context("Failed to fetch issues from Gitea/Forgejo API")?
            .json::<Vec<GiteaIssue>>()
            .context("Failed to parse Gitea/Forgejo API response")?
            .into_iter()
            .filter_map(|i| match i.pull_request {
                Some(_) => None,
                None => Some(i.into()),
            })
            .collect::<Vec<Issue>>();

        Ok(issues)
    }

    fn get_prs(
        &self,
        use_auth: bool,
        author: Option<&str>,
        labels: &[String],
        page: u32,
        per_page: u32,
        state: PrState,
        draft: bool,
    ) -> anyhow::Result<Vec<Pr>> {
        let base_url = self.get_api_base_url()?;
        let repo_path = match self.remote.as_ref() {
            Some(v) => &v.path,
            None => anyhow::bail!("No remote data available"),
        };
        let url = format!("{base_url}/repos/{repo_path}/pulls");
        let request = self
            .http_client
            .get(&url)
            .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
            .query(&[("state", state.clone())])
            .query(&[("page", page)])
            .query(&[("limit", per_page)]);

        let prs: Vec<GiteaPullRequest> = request
            .send()
            .context("Failed to fetch pull requests from Gitea/Forgejo API")?
            .json()
            .context("Failed to parse Gitea/Forgejo API response")?;
        let mut filtered: Vec<GiteaPullRequest> = prs;

        match state {
            PrState::Merged => filtered.retain(|pr| pr.merged),
            PrState::Closed => filtered.retain(|pr| !pr.merged),
            _ => {}
        }

        if let Some(author_name) = author {
            filtered.retain(|pr| pr.user.login == author_name);
        }

        if !labels.is_empty() {
            filtered.retain(|pr| {
                labels
                    .iter()
                    .all(|label| pr.labels.iter().any(|l| &l.name == label))
            });
        }

        if draft {
            filtered.retain(|pr| pr.draft);
        }

        Ok(filtered.into_iter().map(Into::into).collect())
    }

    fn create_pr(
        &self,
        title: &str,
        source_branch: &str,
        target_branch: &str,
        body: Option<&str>,
        draft: bool,
    ) -> anyhow::Result<Pr> {
        let base_url = self.get_api_base_url()?;
        let repo_path = match self.remote.as_ref() {
            Some(v) => &v.path,
            None => anyhow::bail!("No remote data available"),
        };
        let url = format!("{base_url}/repos/{repo_path}/pulls");
        let request_body = serde_json::json!({
            "title": if draft { format!("WIP: {title}") } else { title.to_string() },
            "head": source_branch,
            "base": target_branch,
            "body": body.unwrap_or_default(),
        });

        let pr: GiteaPullRequest = self
            .http_client
            .post(&url)
            .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
            .json(&request_body)
            .send()
            .context("Failed to create pull request on Gitea/Forgejo")?
            .json()
            .context("Failed to parse Gitea/Forgejo API response")?;

        Ok(pr.into())
    }

    fn get_pr_ref(&self, pr_number: u32) -> String {
        format!("pull/{pr_number}/head")
    }

    fn get_web_url(&self, target: WebTarget) -> anyhow::Result<String> {
        let remote = match self.remote.as_ref() {
            Some(v) => v,
            None => anyhow::bail!("No remote data available"),
        };
        let host = &remote.host;
        let path = &remote.path;
        let base_url = match remote.port {
            Some(port) => format!("https://{host}:{port}/{path}"),
            None => format!("https://{host}/{path}"),
        };
        let url = match target {
            WebTarget::Issues => format!("{base_url}/issues"),
            WebTarget::Mrs | WebTarget::Prs => format!("{base_url}/pulls"),
            WebTarget::Repository => base_url,
        };

        Ok(url)
    }
}

/// Gitea/Forgejo API response for issues.
/// https://docs.gitea.com/api/#tag/issue/operation/issueSearchIssues
#[derive(Debug, serde::Deserialize)]
struct GiteaIssue {
    number: u32,
    title: String,
    state: IssueState,
    labels: Vec<GiteaLabel>,
    user: GiteaUser,
    html_url: String,
    pull_request: Option<GiteaIssuePrField>,
}

impl From<GiteaIssue> for Issue {
    fn from(issue: GiteaIssue) -> Self {
        Issue {
            id: issue.number,
            title: issue.title,
            state: issue.state,
            author: issue.user.login,
            url: issue.html_url,
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GiteaLabel {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct GiteaUser {
    login: String,
}

#[derive(Debug, serde::Deserialize)]
struct GiteaIssuePrField {}

/// Gitea/Forgejo API response for pull requests.
/// https://docs.gitea.com/api/#tag/repository/operation/repoNewPinAllowed
#[derive(Debug, serde::Deserialize)]
struct GiteaPullRequest {
    number: u32,
    title: String,
    state: String,
    labels: Vec<GiteaLabel>,
    user: GiteaUser,
    created_at: String,
    updated_at: String,
    html_url: String,
    head: GiteaPrRef,
    base: GiteaPrRef,
    draft: bool,
    merged: bool,
}

impl From<GiteaPullRequest> for Pr {
    fn from(pr: GiteaPullRequest) -> Self {
        Pr {
            id: pr.number,
            title: pr.title,
            state: if pr.merged {
                "merged".to_string()
            } else {
                pr.state
            },
            author: pr.user.login,
            url: pr.html_url,
            labels: pr.labels.into_iter().map(|l| l.name).collect(),
            created_at: pr.created_at,
            updated_at: pr.updated_at,
            source_branch: pr.head.ref_name,
            target_branch: pr.base.ref_name,
            draft: pr.draft,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GiteaPrRef {
    #[serde(rename = "ref")]
    ref_name: String,
}
