use anyhow::Context;
use serde::Deserialize;

use crate::{
    cli::{
        forge::http_client::{
            self, HttpClient, IntoPaginatedResponse, PaginatedResponse, WithAuth, WithHttpStatusOk,
        },
        issue::{CreateIssueOptions, Issue, IssueState, ListIssueFilters},
        pr::{CreatePrOptions, ListPrsFilters, Pr, PrState},
    },
    git::GitRemoteData,
};

const AUTH_TOKEN: &str = "GIT_FORGE_GITEA_TOKEN";
const AUTH_SCHEME: &str = "token";

// =============================================================================
// Domain Types
// =============================================================================

/// Gitea/Forgejo API response for issues.
/// https://docs.gitea.com/api/#tag/issue/operation/issueSearchIssues
#[derive(Debug, Deserialize)]
struct GiteaIssue {
    number: u32,
    title: String,
    state: IssueState,
    labels: Vec<GiteaLabel>,
    user: GiteaUser,
    html_url: String,
    pull_request: Option<GiteaIssuePrField>,
    created_at: String,
    updated_at: String,
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

impl From<GiteaIssue> for Pr {
    fn from(issue: GiteaIssue) -> Self {
        let (draft, merged) = issue
            .pull_request
            .map(|pr| (pr.draft, pr.merged))
            .unwrap_or_default();

        Pr {
            id: issue.number,
            title: issue.title,
            state: match issue.state {
                IssueState::Closed => {
                    if merged {
                        String::from("merged")
                    } else {
                        String::from("closed")
                    }
                }
                _ => String::from("open"),
            },
            author: issue.user.login,
            url: issue.html_url,
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            draft,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GiteaUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GiteaIssuePrField {
    draft: bool,
    merged: bool,
}

/// Gitea/Forgejo API response for pull requests.
/// https://docs.gitea.com/api/#tag/repository/operation/repoNewPinAllowed
#[derive(Debug, Deserialize)]
struct GiteaPullRequest {
    number: u32,
    title: String,
    state: String,
    labels: Vec<GiteaLabel>,
    user: GiteaUser,
    created_at: String,
    updated_at: String,
    html_url: String,
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
            draft: pr.draft,
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
) -> anyhow::Result<PaginatedResponse<Issue>> {
    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let repo_path = &remote.path;
    let endpoint_url = format!("{base_url}/repos/{repo_path}/issues");

    let mut request = http_client
        .get(&endpoint_url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("limit", filters.per_page)])
        .query(&[("type", "issues")]);

    if let Some(assignee) = filters.assignee {
        request = request.query(&[("assigned_by", assignee)]);
    }

    if let Some(author) = filters.author {
        request = request.query(&[("created_by", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
    }

    if let Some(query) = filters.query {
        request = request.query(&[("q", query)]);
    }

    let response = request
        .send()
        .context("Failed to fetch items from Gitea Search API")?
        .with_http_status_ok()?;

    let has_next_page = http_client::has_next_link_header(&response);

    response
        .json()
        .context("Failed to parse Gitea Search API response")
        .map(|res: Vec<GiteaIssue>| res.into_paginated_response(has_next_page))
}

pub fn create_issue(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    options: &CreateIssueOptions,
) -> anyhow::Result<Issue> {
    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/issues");
    let request_body = serde_json::json!({
        "title": options.title,
        "body": options.body,
    });

    eprintln!("Creating issue on Gitea/Forgejo...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating issue on Gitea/Forgejo")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")
        .map(|issue: GiteaIssue| issue.into())
}

pub fn get_prs(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    filters: &ListPrsFilters,
    use_auth: bool,
) -> anyhow::Result<PaginatedResponse<Pr>> {
    // Check for unsupported filters
    if filters.draft {
        anyhow::bail!("Gitea/Forgejo does not support filtering by draft status");
    }

    if matches!(filters.state, PrState::Merged) {
        anyhow::bail!(
            "Gitea/Forgejo does not support filtering by merged state. Use --state=closed to see both closed and merged PRs"
        );
    }

    let base_url = match api_url {
        Some(url) => url,
        None => &build_api_base_url(remote),
    };
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/issues");

    let mut request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("type", "pulls")])
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("limit", filters.per_page)]);

    if let Some(author) = filters.author {
        request = request.query(&[("created_by", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
    }

    if let Some(query) = filters.query {
        request = request.query(&[("q", query)]);
    }

    let response = request
        .send()
        .context("Network request failed while fetching pull requests from Gitea/Forgejo")?
        .with_http_status_ok()?;

    let has_next_page = http_client::has_next_link_header(&response);

    response
        .json()
        .context("Failed to parse API response")
        .map(|items: Vec<GiteaIssue>| {
            items
                .into_iter()
                .map(Into::into)
                .collect::<Vec<Pr>>()
                .into_paginated_response(has_next_page)
        })
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
        "title": if options.draft { format!("WIP: {}", options.title) } else { options.title.to_string() },
        "head": options.source_branch,
        "base": options.target_branch,
        "body": options.body,
    });

    eprintln!("Creating pull request on Gitea/Forgejo...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating pull request on Gitea/Forgejo")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")
        .map(|pr: GiteaPullRequest| pr.into())
}

pub fn get_pr_ref(pr_number: u32) -> String {
    format!("pull/{pr_number}/head")
}

pub fn get_url_for_home(remote: &GitRemoteData) -> String {
    build_web_base_url(remote)
}

pub fn get_url_for_commit(remote: &GitRemoteData, commit: &str) -> String {
    format!("{}/commit/{}", build_web_base_url(remote), commit)
}

pub fn get_url_for_issue(remote: &GitRemoteData, issue_number: u32) -> String {
    format!("{}/issues/{}", build_web_base_url(remote), issue_number)
}

pub fn get_url_for_issues(remote: &GitRemoteData) -> String {
    format!("{}/issues", build_web_base_url(remote))
}

pub fn get_url_for_issue_creation(remote: &GitRemoteData) -> String {
    format!("{}/issues/new", build_web_base_url(remote))
}

pub fn get_url_for_pr(remote: &GitRemoteData, pr_number: u32) -> String {
    format!("{}/pulls/{}", build_web_base_url(remote), pr_number)
}

pub fn get_url_for_pr_creation(
    remote: &GitRemoteData,
    target_branch: &str,
    source_branch: &str,
) -> String {
    let base_url = build_web_base_url(remote);

    format!("{base_url}/compare/{target_branch}...{source_branch}")
}

pub fn get_url_for_prs(remote: &GitRemoteData) -> String {
    format!("{}/pulls", build_web_base_url(remote))
}

pub fn get_url_for_releases(remote: &GitRemoteData) -> String {
    format!("{}/releases", build_web_base_url(remote))
}

pub fn get_url_for_path(
    remote: &GitRemoteData,
    path: &str,
    commit: &str,
    line_number: Option<u32>,
) -> String {
    let base = build_web_base_url(remote);
    let mut url = format!("{}/src/commit/{}/{}", base, commit, path);

    if let Some(line) = line_number {
        url.push_str(&format!("#L{}", line));
    }

    url
}

// =============================================================================
// Private Helpers
// =============================================================================

fn build_api_base_url(remote: &GitRemoteData) -> String {
    let (host, port) = (&remote.host, remote.port);

    match port {
        Some(p) => format!("https://{host}:{p}/api/v1"),
        None => format!("https://{host}/api/v1"),
    }
}

fn build_web_base_url(remote: &GitRemoteData) -> String {
    let host = &remote.host;
    let path = &remote.path;

    match remote.port {
        Some(port) => format!("https://{host}:{port}/{path}"),
        None => format!("https://{host}/{path}"),
    }
}
