use anyhow::Context;
use serde::Deserialize;

use crate::{
    cli::{
        forge::http_client::{HttpClient, PaginatedResponse, WithAuth, WithHttpStatusOk},
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

#[derive(Debug, Deserialize)]
struct GiteaLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GiteaUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GiteaIssuePrField {}

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

#[derive(Debug, Deserialize)]
struct GiteaPrRef {
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

    let response: PaginatedResponse<GiteaIssue> = request
        .send()
        .context("Network request failed while fetching issues from Gitea/Forgejo")?
        .with_http_status_ok()?
        .try_into()?;

    let filtered_response = response.filter_map(|i| match i.pull_request {
        Some(_) => None,
        None => Some(i.into()),
    });

    Ok(filtered_response)
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

    let issue: GiteaIssue = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body)
        .send()
        .context("Network request failed while creating issue on Gitea/Forgejo")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")?;

    Ok(issue.into())
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
    let repo_path = &remote.path;
    let url = format!("{base_url}/repos/{repo_path}/pulls");
    let request = http_client
        .get(&url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("limit", filters.per_page)]);

    let response: PaginatedResponse<GiteaPullRequest> = request
        .send()
        .context("Network request failed while fetching pull requests from Gitea/Forgejo")?
        .with_http_status_ok()?
        .try_into()?;

    // Client-side filtering
    let filtered_response = response.filter_map(|pr| {
        let state_matches = match filters.state {
            PrState::Merged => pr.merged,
            PrState::Closed => !pr.merged,
            _ => true,
        };

        let author_matches = filters
            .author
            .map(|author_name| pr.user.login == author_name)
            .unwrap_or(true);

        let labels_match = filters.labels.is_empty()
            || filters
                .labels
                .iter()
                .all(|label| pr.labels.iter().any(|l| &l.name == label));

        let draft_matches = !filters.draft || pr.draft;

        if state_matches && author_matches && labels_match && draft_matches {
            Some(pr.into())
        } else {
            None
        }
    });

    Ok(filtered_response)
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

    let pr: GiteaPullRequest = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body)
        .send()
        .context("Network request failed while creating pull request on Gitea/Forgejo")?
        .with_http_status_ok()?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")?;

    Ok(pr.into())
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
