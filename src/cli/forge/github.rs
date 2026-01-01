use anyhow::Context;
use serde::Deserialize;

use crate::{
    cli::{
        forge::http_client::{HttpClient, WithAuth},
        issue::{Issue, IssueState, ListIssueFilters},
        pr::{CreatePrOptions, ListPrsFilters, Pr, PrState},
        web::WebTarget,
    },
    git::GitRemoteData,
};

const AUTH_TOKEN: &str = "GITHUB_TOKEN";
const AUTH_SCHEME: &str = "Bearer";

// =============================================================================
// Domain Types
// =============================================================================

/// GitHub API response for issues.
/// https://docs.github.com/en/rest/issues/issues
#[derive(Debug, Deserialize)]
struct GitHubIssue {
    number: u32,
    title: String,
    state: IssueState,
    labels: Vec<GitHubLabel>,
    user: GitHubUser,
    html_url: String,
    pull_request: Option<GitHubIssuePrField>,
}

impl From<GitHubIssue> for Issue {
    fn from(issue: GitHubIssue) -> Self {
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

#[derive(Debug, Deserialize)]
struct GitHubLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubIssuePrField {}

/// GitHub API response for pull requests.
/// https://docs.github.com/en/rest/pulls/pulls
#[derive(Debug, Deserialize)]
struct GitHubPullRequest {
    number: u32,
    title: String,
    state: String,
    labels: Vec<GitHubLabel>,
    user: GitHubUser,
    created_at: String,
    updated_at: String,
    html_url: String,
    head: GitHubPrRef,
    base: GitHubPrRef,
    draft: Option<bool>,
    merged_at: Option<String>,
}

impl From<GitHubPullRequest> for Pr {
    fn from(pr: GitHubPullRequest) -> Self {
        Pr {
            id: pr.number,
            title: pr.title,
            state: if pr.merged_at.is_some() {
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
            draft: pr.draft.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubPrRef {
    #[serde(rename = "ref")]
    ref_name: String,
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
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/issues");
    let mut request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github.v3+json")
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("per_page", filters.per_page)]);

    if let Some(author) = filters.author {
        request = request.query(&[("creator", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
    }

    let issues = request
        .send()
        .context("Failed to fetch issues from GitHub API")?
        .json::<Vec<GitHubIssue>>()
        .context("Failed to parse GitHub API response")?
        .into_iter()
        .filter_map(|i| match i.pull_request {
            Some(_) => None,
            None => Some(i.into()),
        })
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
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/pulls");
    let request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github.v3+json")
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("per_page", filters.per_page)]);

    let prs: Vec<GitHubPullRequest> = request
        .send()
        .context("Failed to fetch pull requests from GitHub API")?
        .json()
        .context("Failed to parse GitHub API response")?;
    let mut filtered: Vec<GitHubPullRequest> = prs;

    match filters.state {
        PrState::Merged => filtered.retain(|pr| pr.merged_at.is_some()),
        PrState::Closed => filtered.retain(|pr| pr.merged_at.is_none()),
        _ => {}
    }

    if let Some(author_name) = filters.author {
        filtered.retain(|pr| pr.user.login == author_name);
    }

    if !filters.labels.is_empty() {
        filtered.retain(|pr| {
            filters
                .labels
                .iter()
                .all(|label| pr.labels.iter().any(|l| &l.name == label))
        });
    }

    if filters.draft {
        filtered.retain(|pr| pr.draft.unwrap_or(false));
    }

    Ok(filtered.into_iter().map(Into::into).collect())
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
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/pulls");
    let request_body = serde_json::json!({
        "title": options.title,
        "head": options.source_branch,
        "base": options.target_branch,
        "body": options.body.unwrap_or_default(),
        "draft": options.draft,
    });

    let pr: GitHubPullRequest = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github.v3+json")
        .json(&request_body)
        .send()
        .context("Failed to create pull request on GitHub")?
        .json()
        .context("Failed to parse GitHub API response")?;

    Ok(pr.into())
}

pub fn get_pr_ref(pr_number: u32) -> String {
    format!("pull/{pr_number}/head")
}

pub fn build_web_url(remote: &GitRemoteData, target: &WebTarget) -> String {
    let host = &remote.host;
    let path = &remote.path;
    let repo_url = match remote.port {
        Some(port) => format!("https://{host}:{port}/{path}"),
        None => format!("https://{host}/{path}"),
    };

    match target {
        WebTarget::Issues => format!("{repo_url}/issues"),
        WebTarget::Mrs | WebTarget::Prs => format!("{repo_url}/pulls"),
        WebTarget::Repository => repo_url,
    }
}

// =============================================================================
// Private Helpers
// =============================================================================

fn build_api_base_url(remote: &GitRemoteData) -> String {
    let (host, port) = (&remote.host, remote.port);

    if host == "github.com" {
        "https://api.github.com".to_string()
    } else {
        match port {
            Some(p) => format!("https://{host}:{p}/api/v3"),
            None => format!("https://{host}/api/v3"),
        }
    }
}
