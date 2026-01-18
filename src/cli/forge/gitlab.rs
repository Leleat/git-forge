use anyhow::Context;
use serde::Deserialize;
use url::form_urlencoded::byte_serialize;

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

const AUTH_TOKEN: &str = "GIT_FORGE_GITLAB_TOKEN";
const AUTH_SCHEME: &str = "Bearer";

// =============================================================================
// Domain Types
// =============================================================================

/// GitLab API response for issues.
/// https://docs.gitlab.com/api/issues/#list-project-issues
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct GitLabUser {
    username: String,
}

/// GitLab API response for pull requests.
/// https://docs.gitlab.com/api/merge_requests/#list-project-merge-requests
#[derive(Debug, Deserialize)]
struct GitLabMergeRequest {
    iid: u32,
    title: String,
    state: String,
    labels: Vec<String>,
    author: GitLabUser,
    created_at: String,
    updated_at: String,
    web_url: String,
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
) -> anyhow::Result<PaginatedResponse<Issue>> {
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

    if let Some(assignee) = filters.assignee {
        request = request.query(&[("assignee_username", assignee)]);
    }

    if let Some(author) = filters.author {
        request = request.query(&[("author_username", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
    }

    if let Some(query) = filters.query {
        request = request.query(&[("search", query)]);
    }

    let response = request
        .send()
        .context("Network request failed while fetching issues from GitLab")?
        .with_http_status_ok()?;

    let has_next_page = http_client::has_next_link_header(&response);

    response
        .json()
        .context("Failed to parse GitHub Search API response")
        .map(|vec: Vec<GitLabIssue>| vec.into_paginated_response(has_next_page))
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
    let encoded_path = byte_serialize(remote.path.as_bytes()).collect::<String>();
    let url = format!("{base_url}/projects/{encoded_path}/issues");
    let request_body = serde_json::json!({
        "title": options.title,
        "description": options.body,
    });

    eprintln!("Creating issue on GitLab...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating issue on GitLab")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse GitLab API response")
        .map(|issue: GitLabIssue| issue.into())
}

pub fn get_prs(
    http_client: &HttpClient,
    remote: &GitRemoteData,
    api_url: Option<&str>,
    filters: &ListPrsFilters,
    use_auth: bool,
) -> anyhow::Result<PaginatedResponse<Pr>> {
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

    if let Some(query) = filters.query {
        request = request.query(&[("search", query)]);
    }

    if filters.draft {
        request = request.query(&[("wip", "yes")]);
    }

    let response = request
        .send()
        .context("Network request failed while fetching merge requests from GitLab")?
        .with_http_status_ok()?;

    let has_next_page = http_client::has_next_link_header(&response);

    response
        .json()
        .context("Failed to parse GitHub Search API response")
        .map(|vec: Vec<GitLabMergeRequest>| vec.into_paginated_response(has_next_page))
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
        "description": options.body,
    });

    eprintln!("Creating merge request on GitLab...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating merge request on GitLab")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse GitLab API response")
        .map(|mr: GitLabMergeRequest| mr.into())
}

pub fn get_pr_ref(pr_number: u32) -> String {
    format!("merge-requests/{pr_number}/head")
}

pub fn get_url_for_home(remote: &GitRemoteData) -> String {
    build_web_base_url(remote)
}

pub fn get_url_for_commit(remote: &GitRemoteData, commit: &str) -> String {
    format!("{}/-/commit/{}", build_web_base_url(remote), commit)
}

pub fn get_url_for_issue(remote: &GitRemoteData, issue_number: u32) -> String {
    format!("{}/-/issues/{}", build_web_base_url(remote), issue_number)
}

pub fn get_url_for_issues(remote: &GitRemoteData) -> String {
    format!("{}/-/issues", build_web_base_url(remote))
}

pub fn get_url_for_issue_creation(remote: &GitRemoteData) -> String {
    format!("{}/-/issues/new", build_web_base_url(remote))
}

pub fn get_url_for_pr(remote: &GitRemoteData, pr_number: u32) -> String {
    format!(
        "{}/-/merge_requests/{}",
        build_web_base_url(remote),
        pr_number
    )
}

pub fn get_url_for_prs(remote: &GitRemoteData) -> String {
    format!("{}/-/merge_requests", build_web_base_url(remote))
}

pub fn get_url_for_releases(remote: &GitRemoteData) -> String {
    format!("{}/-/releases", build_web_base_url(remote))
}

pub fn get_url_for_path(
    remote: &GitRemoteData,
    path: &str,
    commit: &str,
    line_number: Option<u32>,
) -> String {
    let base = build_web_base_url(remote);
    let mut url = format!("{base}/-/blob/{commit}/{path}");

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
        Some(p) => format!("https://{host}:{p}/api/v4"),
        None => format!("https://{host}/api/v4"),
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
