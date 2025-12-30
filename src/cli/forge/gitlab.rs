use anyhow::Context;
use url::form_urlencoded::byte_serialize;

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

const AUTH_TOKEN: &str = "GITLAB_TOKEN";
const AUTH_SCHEME: &str = "Bearer";

pub struct GitLabClient {
    api_url: Option<String>,
    remote: Option<GitRemoteData>,
    http_client: HttpClient,
}

impl GitLabClient {
    pub fn new(remote: Option<GitRemoteData>, api_url: Option<String>) -> Self {
        GitLabClient {
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
            Some(p) => format!("https://{host}:{p}/api/v4"),
            None => format!("https://{host}/api/v4"),
        };

        Ok(base_url)
    }
}

impl ForgeClient for GitLabClient {
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
        let encoded_path: String = match self.remote.as_ref() {
            Some(v) => byte_serialize(v.path.as_bytes()).collect(),
            None => anyhow::bail!("No remote data available"),
        };
        let url = format!("{base_url}/projects/{encoded_path}/issues");
        let state = match state {
            IssueState::Open => "opened".to_string(),
            _ => state.to_string(),
        };
        let mut request = self
            .http_client
            .get(&url)
            .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
            .query(&[("state", state)])
            .query(&[("page", page)])
            .query(&[("per_page", per_page)]);

        if let Some(author) = author {
            request = request.query(&[("author_username", author)]);
        }

        if !labels.is_empty() {
            request = request.query(&[("labels", labels.join(","))]);
        }

        let issues = request
            .send()
            .context("Failed to fetch issues from GitLab API")?
            .json::<Vec<GitLabIssue>>()
            .context("Failed to parse GitLab API response")?
            .into_iter()
            .map(Into::into)
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
        let remote = match self.remote.as_ref() {
            Some(v) => v,
            None => anyhow::bail!("No remote data available"),
        };
        let encoded_path: String = byte_serialize(remote.path.as_bytes()).collect();
        let url = format!("{base_url}/projects/{encoded_path}/merge_requests");
        let state = match state {
            PrState::Open => "opened".to_string(),
            _ => state.to_string(),
        };
        let mut request = self
            .http_client
            .get(&url)
            .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
            .query(&[("state", state)])
            .query(&[("page", page)])
            .query(&[("per_page", per_page)]);

        if let Some(author_name) = author {
            request = request.query(&[("author_username", author_name)]);
        }

        if !labels.is_empty() {
            request = request.query(&[("labels", labels.join(","))]);
        }

        if draft {
            request = request.query(&[("wip", "yes")]);
        }

        let mrs: Vec<GitLabMergeRequest> = request
            .send()
            .context("Failed to fetch merge requests from GitLab API")?
            .json()
            .context("Failed to parse GitLab API response")?;

        Ok(mrs.into_iter().map(Into::into).collect())
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
        let remote = match self.remote.as_ref() {
            Some(v) => v,
            None => anyhow::bail!("No remote data available"),
        };
        let encoded_path: String = byte_serialize(remote.path.as_bytes()).collect();
        let url = format!("{base_url}/projects/{encoded_path}/merge_requests");
        let request_body = serde_json::json!({
            "source_branch": source_branch,
            "target_branch": target_branch,
            "title": if draft { format!("Draft: {title}") } else { title.to_string() },
            "description": body.unwrap_or_default(),
        });

        let mr: GitLabMergeRequest = self
            .http_client
            .post(&url)
            .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
            .json(&request_body)
            .send()
            .context("Failed to create merge request on GitLab")?
            .json()
            .context("Failed to parse GitLab API response")?;

        Ok(mr.into())
    }

    fn get_pr_ref(&self, pr_number: u32) -> String {
        format!("merge-requests/{pr_number}/head")
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
            WebTarget::Issues => format!("{base_url}/-/issues"),
            WebTarget::Mrs | WebTarget::Prs => format!("{base_url}/-/merge_requests"),
            WebTarget::Repository => base_url,
        };

        Ok(url)
    }
}

/// GitLab API response for issues.
/// https://docs.gitlab.com/api/issues/#list-project-issues
#[derive(Debug, serde::Deserialize)]
struct GitLabIssue {
    iid: u32,
    title: String,
    state: String,
    labels: Vec<String>,
    author: GitLabUser,
    web_url: String,
}

impl From<GitLabIssue> for Issue {
    fn from(issue: GitLabIssue) -> Self {
        let state = if issue.state == "opened" {
            IssueState::Open
        } else if issue.state == "closed" {
            IssueState::Closed
        } else {
            IssueState::All
        };

        Issue {
            id: issue.iid,
            author: issue.author.username,
            labels: issue.labels,
            state,
            title: issue.title,
            url: issue.web_url,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GitLabUser {
    username: String,
}

/// GitLab API response for pull requests.
/// https://docs.gitlab.com/api/merge_requests/#list-project-merge-requests
#[derive(Debug, serde::Deserialize)]
struct GitLabMergeRequest {
    iid: u32,
    title: String,
    state: String,
    labels: Vec<String>,
    author: GitLabUser,
    created_at: String,
    updated_at: String,
    web_url: String,
    source_branch: String,
    target_branch: String,
    draft: bool,
}

impl From<GitLabMergeRequest> for Pr {
    fn from(mr: GitLabMergeRequest) -> Self {
        Pr {
            id: mr.iid,
            title: mr.title,
            state: if mr.state == "opened" {
                "open".to_string()
            } else {
                mr.state
            },
            author: mr.author.username,
            url: mr.web_url,
            labels: mr.labels,
            created_at: mr.created_at,
            updated_at: mr.updated_at,
            source_branch: mr.source_branch,
            target_branch: mr.target_branch,
            draft: mr.draft,
        }
    }
}
