//! The `issue` subcommand.

use anyhow::Context;
use clap::{Args, Subcommand};

use crate::{
    cli::forge::{self, ApiType, HttpClient, gitea, github, gitlab},
    git,
};

// =============================================================================
// CLI Arguments
// =============================================================================

const DEFAULT_PER_PAGE: u32 = 30;

/// Command-line arguments for the `issue` subcommand.
#[derive(Args)]
pub struct IssueCommandArgs {
    #[command(subcommand)]
    pub subcommand: IssueCommand,
}

/// Available subcommands for issue subcommand.
#[derive(Subcommand)]
pub enum IssueCommand {
    /// List issues as TSV.
    #[command(alias = "l", about = "List issues as TSV")]
    List(IssueListCommandArgs),
}

/// Command-line arguments for listing issues.
#[derive(Args)]
pub struct IssueListCommandArgs {
    #[arg(
        long,
        value_name = "TYPE",
        help = "Specify the forge which affects the API schema etc."
    )]
    pub api: Option<ApiType>,

    #[arg(
        long,
        help = "Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4) instead of relying on the auto-detection"
    )]
    pub api_url: Option<String>,

    #[arg(
        long,
        help = "Use authentication with environment variables (GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN)"
    )]
    pub auth: bool,

    #[arg(long, help = "Filter by author username")]
    pub author: Option<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Columns to include in TSV output (comma-separated)"
    )]
    pub columns: Vec<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Filter by labels (comma-separated)"
    )]
    pub labels: Vec<String>,

    #[arg(
        long,
        default_value_t = 1,
        value_name = "NUMBER",
        help = "Page number to fetch"
    )]
    pub page: u32,

    #[arg(
        long,
        default_value_t = DEFAULT_PER_PAGE,
        value_name = "NUMBER",
        help = "Number of issues per page"
    )]
    pub per_page: u32,

    #[arg(long, default_value = "origin", help = "Git remote to use")]
    pub remote: String,

    #[arg(long, help = "Filter by state")]
    pub state: Option<IssueState>,
}

// =============================================================================
// Domain Types
// =============================================================================

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum IssueState {
    /// Open issues that haven't been closed yet.
    Open,
    /// Closed issues that have been resolved.
    Closed,
    /// All issues regardless of state.
    All,
}

impl std::fmt::Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueState::Open => write!(f, "open"),
            IssueState::Closed => write!(f, "closed"),
            IssueState::All => write!(f, "all"),
        }
    }
}

/// An issue from a git forge.
pub struct Issue {
    /// The issue number (e.g., #42).
    pub id: u32,
    /// The issue title.
    pub title: String,
    /// The current state (open, closed, etc.).
    pub state: IssueState,
    /// The username of the issue author.
    pub author: String,
    /// The web URL to view this issue.
    pub url: String,
    /// Labels attached to this issue.
    pub labels: Vec<String>,
}

pub struct ListIssueFilters<'a> {
    pub author: Option<&'a str>,
    pub labels: &'a [String],
    pub page: u32,
    pub per_page: u32,
    pub state: &'a IssueState,
}

// =============================================================================
// Command Logic
// =============================================================================

/// Lists issues from the remote repository's forge and outputs them as TSV.
pub fn list_issues(args: IssueListCommandArgs) -> anyhow::Result<()> {
    let http_client = HttpClient::new();
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };
    let get_issues = match api_type {
        ApiType::GitHub => github::get_issues,
        ApiType::GitLab => gitlab::get_issues,
        ApiType::Gitea | ApiType::Forgejo => gitea::get_issues,
    };
    let issue_filters = ListIssueFilters {
        author: args.author.as_deref(),
        labels: &args.labels,
        page: args.page,
        per_page: args.per_page,
        state: &args.state.unwrap_or(IssueState::Open),
    };
    let issues = get_issues(
        &http_client,
        &remote,
        args.api_url.as_deref(),
        &issue_filters,
        args.auth,
    )
    .context("Failed fetching issues")?;

    let output = format_issues_to_tsv(
        &issues,
        if args.columns.is_empty() {
            vec!["id".to_string(), "title".to_string(), "url".to_string()]
        } else {
            args.columns
        },
    );

    if !output.is_empty() {
        println!("{output}");
    }

    Ok(())
}

// =============================================================================
// Private Helpers
// =============================================================================

fn format_issues_to_tsv(issues: &[Issue], columns: Vec<String>) -> String {
    issues
        .iter()
        .map(|issue| {
            columns
                .iter()
                .map(|col| get_column_value_for_issue(col, issue))
                .collect::<Vec<String>>()
                .join("\t")
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_column_value_for_issue(column: &str, issue: &Issue) -> String {
    match column {
        "id" => issue.id.to_string(),
        "title" => escape_tsv(&issue.title),
        "state" => issue.state.to_string(),
        "labels" => escape_tsv(&issue.labels.join(",")),
        "author" => escape_tsv(&issue.author),
        "url" => issue.url.clone(),
        _ => String::new(),
    }
}

fn escape_tsv(value: &str) -> String {
    value
        .replace('\t', " ")
        .replace("\r\n", " ")
        .replace('\n', " ")
        .trim()
        .to_string()
}
