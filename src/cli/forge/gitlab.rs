use anyhow::Context;
use url::form_urlencoded::byte_serialize;

use crate::{
    cli::{
        forge::http_client::{HttpClient, WithAuth},
        issue::{Issue, IssueState, ListIssueFilters},
        pr::{CreatePrOptions, ListPrsFilters, Pr, PrState},
    },
    git::GitRemoteData,
};

const AUTH_TOKEN: &str = "GITLAB_TOKEN";
const AUTH_SCHEME: &str = "Bearer";

// =============================================================================
// Domain Types
// =============================================================================

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

// =============================================================================
// Command Logic
// =============================================================================

pub fn get_issues(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    filters: &ListIssueFilters,
    use_auth: bool,
) -> anyhow::Result<Vec<Issue>> {
    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let encoded_path = byte_serialize(remote.path.as_bytes()).collect::<String>();
    let url = format!("{base_url}/projects/{encoded_path}/issues");
    let state = match filters.state {
        IssueState::Open => "opened".to_string(),
        _ => filters.state.to_string(),
    };
    let mut request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("state", state)])
        .query(&[("page", filters.page)])
        .query(&[("per_page", filters.per_page)]);

    if let Some(author) = filters.author {
        request = request.query(&[("author_username", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
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

pub fn get_prs(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    filters: &ListPrsFilters,
    use_auth: bool,
) -> anyhow::Result<Vec<Pr>> {
    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let encoded_path = byte_serialize(remote.path.as_bytes()).collect::<String>();
    let url = format!("{base_url}/projects/{encoded_path}/merge_requests");
    let state = match filters.state {
        PrState::Open => "opened".to_string(),
        _ => filters.state.to_string(),
    };
    let mut request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("state", state)])
        .query(&[("page", filters.page)])
        .query(&[("per_page", filters.per_page)]);

    if let Some(author_name) = filters.author {
        request = request.query(&[("author_username", author_name)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
    }

    if filters.draft {
        request = request.query(&[("wip", "yes")]);
    }

    let mrs: Vec<GitLabMergeRequest> = request
        .send()
        .context("Failed to fetch merge requests from GitLab API")?
        .json()
        .context("Failed to parse GitLab API response")?;

    Ok(mrs.into_iter().map(Into::into).collect())
}

pub fn create_pr(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    options: &CreatePrOptions,
) -> anyhow::Result<Pr> {
    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let encoded_path = byte_serialize(remote.path.as_bytes()).collect::<String>();
    let url = format!("{base_url}/projects/{encoded_path}/merge_requests");
    let request_body = serde_json::json!({
        "source_branch": options.source_branch,
        "target_branch": options.target_branch,
        "title": if options.draft { format!("Draft: {}", options.title) } else { options.title.to_string() },
        "description": options.body.unwrap_or_default(),
    });

    let mr: GitLabMergeRequest = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body)
        .send()
        .context("Failed to create merge request on GitLab")?
        .json()
        .context("Failed to parse GitLab API response")?;

    Ok(mr.into())
}

pub fn get_pr_ref(pr_number: u32) -> String {
    format!("merge-requests/{pr_number}/head")
}

// =============================================================================
// Private Helpers
// =============================================================================

fn build_api_base_url(remote: &GitRemoteData) -> String {
    let (host, port) = (&remote.host, remote.port);

    match port {
        Some(p) => format!("https://{host}:{p}/api/v4"),
        None => format!("https://{host}/api/v4"),
    }
}
