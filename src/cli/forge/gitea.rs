use anyhow::Context;

use crate::{
    cli::{
        forge::http_client::{HttpClient, WithAuth},
        issue::{Issue, IssueState, ListIssueFilters},
        pr::{CreatePrOptions, ListPrsFilters, Pr, PrState},
    },
    git::GitRemoteData,
};

const AUTH_TOKEN: &str = "GITEA_TOKEN";
const AUTH_SCHEME: &str = "token";

// =============================================================================
// Domain Types
// =============================================================================

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
    let endpoint_url = format!("{base_url}/repos/{repo_path}/issues");
    let mut request = http_client
        .get(&endpoint_url)
        .with_auth(use_auth, AUTH_TOKEN, AUTH_SCHEME)?
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("limit", filters.per_page)])
        .query(&[("type", "issues")]);

    if let Some(author) = filters.author {
        request = request.query(&[("created_by", author)]);
    }

    if !filters.labels.is_empty() {
        request = request.query(&[("labels", filters.labels.join(","))]);
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
        .query(&[("state", filters.state)])
        .query(&[("page", filters.page)])
        .query(&[("limit", filters.per_page)]);

    let prs: Vec<GiteaPullRequest> = request
        .send()
        .context("Failed to fetch pull requests from Gitea/Forgejo API")?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")?;
    let mut filtered: Vec<GiteaPullRequest> = prs;

    match filters.state {
        PrState::Merged => filtered.retain(|pr| pr.merged),
        PrState::Closed => filtered.retain(|pr| !pr.merged),
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
        filtered.retain(|pr| pr.draft);
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
        "title": if options.draft { format!("WIP: {}", options.title) } else { options.title.to_string() },
        "head": options.source_branch,
        "base": options.target_branch,
        "body": options.body.unwrap_or_default(),
    });

    let pr: GiteaPullRequest = http_client
        .post(&url)
        .with_auth(true, AUTH_TOKEN, AUTH_SCHEME)?
        .json(&request_body)
        .send()
        .context("Failed to create pull request on Gitea/Forgejo")?
        .json()
        .context("Failed to parse Gitea/Forgejo API response")?;

    Ok(pr.into())
}

pub fn get_pr_ref(pr_number: u32) -> String {
    format!("pull/{pr_number}/head")
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
