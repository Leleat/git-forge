use anyhow::Context;
use serde::{Deserialize, de::DeserializeOwned};

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

const AUTH_TOKEN: &str = "GIT_FORGE_GITHUB_TOKEN";
const AUTH_SCHEME: &str = "Bearer";

// =============================================================================
// Domain Types
// =============================================================================

/// GitHub Search API response
/// https://docs.github.com/en/rest/search/search?apiVersion=2022-11-28#search-issues-and-pull-requests
#[derive(Debug, Deserialize)]
struct GitHubSearchResponse<T> {
    items: Vec<T>,
}

impl<S, T: From<S>> IntoPaginatedResponse<T> for GitHubSearchResponse<S> {
    fn into_paginated_response(self, has_next_page: bool) -> PaginatedResponse<T> {
        PaginatedResponse::new(
            self.items.into_iter().map(Into::into).collect(),
            has_next_page,
        )
    }
}

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
            draft: pr.draft.unwrap_or(false),
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
    let url = format!("{base_url}/search/issues");
    let query_string = build_issue_search_query(&remote.path, filters);

    find_items_with_search_api::<GitHubIssue, Issue>(
        http_client,
        &url,
        &query_string,
        filters.page,
        filters.per_page,
        use_auth,
    )
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

    eprintln!("Creating issue on GitHub...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github+json")
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating issue on GitHub")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse GitHub API response")
        .map(|issue: GitHubIssue| issue.into())
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
    let url = format!("{base_url}/search/issues");
    let query_string = build_pr_search_query(&remote.path, filters);

    find_items_with_search_api::<GitHubPullRequest, Pr>(
        http_client,
        &url,
        &query_string,
        filters.page,
        filters.per_page,
        use_auth,
    )
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
        "body": options.body,
        "draft": options.draft,
    });

    eprintln!("Creating pull request on GitHub...");

    let request = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github+json")
        .json(&request_body);

    request
        .send()
        .context("Network request failed while creating pull request on GitHub")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse GitHub API response")
        .map(|pr: GitHubPullRequest| pr.into())
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
    format!("{}/pull/{}", build_web_base_url(remote), pr_number)
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
    let mut url = format!("{}/blob/{}/{}", base, commit, path);

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

    if host == "github.com" {
        "https://api.github.com".to_string()
    } else {
        match port {
            Some(p) => format!("https://{host}:{p}/api/v3"),
            None => format!("https://{host}/api/v3"),
        }
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

/// https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests
fn build_issue_search_query(repo_path: &str, filters: &ListIssueFilters) -> String {
    let mut query_string = match filters.query {
        Some(query) => format!("{query} in:title in:body repo:{repo_path} is:issue"),
        None => format!("repo:{repo_path} is:issue"),
    };

    match filters.state {
        IssueState::Open => query_string.push_str(" is:open"),
        IssueState::Closed => query_string.push_str(" is:closed"),
        IssueState::All => {}
    }

    if let Some(assignee) = filters.assignee {
        query_string.push_str(" assignee:");
        query_string.push_str(assignee);
    }

    if let Some(author) = filters.author {
        query_string.push_str(" author:");
        query_string.push_str(author);
    }

    for label in filters.labels {
        query_string.push_str(" label:");
        query_string.push_str(label);
    }

    query_string
}

/// https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests
fn build_pr_search_query(repo_path: &str, filters: &ListPrsFilters) -> String {
    let mut query_string = match filters.query {
        Some(query) => format!("{query} in:title in:body repo:{repo_path} is:pr"),
        None => format!("repo:{repo_path} is:pr"),
    };

    match filters.state {
        PrState::Open => query_string.push_str(" is:open"),
        PrState::Closed => query_string.push_str(" is:closed is:unmerged"),
        PrState::Merged => query_string.push_str(" is:merged"),
        PrState::All => {}
    }

    if let Some(author) = filters.author {
        query_string.push_str(" author:");
        query_string.push_str(author);
    }

    for label in filters.labels {
        query_string.push_str(" label:");
        query_string.push_str(label);
    }

    if filters.draft {
        query_string.push_str(" draft:true");
    }

    query_string
}

fn find_items_with_search_api<T, U>(
    http_client: &HttpClient,
    url: &str,
    query_string: &str,
    page: u32,
    per_page: u32,
    use_auth: bool,
) -> anyhow::Result<PaginatedResponse<U>>
where
    T: DeserializeOwned,
    U: From<T>,
{
    let request = http_client
        .get(url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .header("Accept", "application/vnd.github+json")
        .query(&[("q", query_string)])
        .query(&[("page", page)])
        .query(&[("per_page", per_page)]);

    let response = request
        .send()
        .context("Failed to fetch items from GitHub Search API")?
        .with_http_status_ok()?;

    let has_next_page = http_client::has_next_link_header(&response);

    response
        .json()
        .context("Failed to parse GitHub Search API response")
        .map(|res: GitHubSearchResponse<T>| res.into_paginated_response(has_next_page))
}
