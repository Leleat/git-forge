//! The `issue` subcommand.

use anyhow::Context;
use clap::{Args, Subcommand};

use crate::{
    cli::forge::{self, ApiType, HttpClient, gitea, github, gitlab},
    git::{self, GitRemoteData},
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
    #[command(alias = "ls")]
    List(IssueListCommandArgs),

    /// Create an issue and open it in the web browser.
    #[command(alias = "cr")]
    Create(IssueCreateCommandArgs),
}

/// Command-line arguments for listing issues.
#[derive(Args)]
pub struct IssueListCommandArgs {
    /// Specify the forge which affects the API schema etc
    #[arg(long, value_name = "TYPE")]
    pub api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4) instead of relying on the auto-detection
    #[arg(long)]
    pub api_url: Option<String>,

    /// Use authentication with environment variables (GITHUB_TOKEN, GITLAB_TOKEN, GITEA_TOKEN)
    #[arg(long)]
    pub auth: bool,

    #[arg(long, help = "Filter by author username")]
    pub author: Option<String>,

    /// Columns to include in TSV output (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub columns: Vec<String>,

    /// Filter by labels (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub labels: Vec<String>,

    /// Page number to fetch
    #[arg(long, default_value_t = 1, value_name = "NUMBER")]
    pub page: u32,

    /// Number of issues per page
    #[arg(long, short_alias = 'l', alias = "limit", default_value_t = DEFAULT_PER_PAGE, value_name = "NUMBER")]
    pub per_page: u32,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    pub remote: String,

    /// Filter by state
    #[arg(long)]
    pub state: Option<IssueState>,
}

/// Command-line arguments for creating an issue.
#[derive(Args)]
pub struct IssueCreateCommandArgs {
    /// Specify the forge which affects the API schema etc.
    #[arg(long, value_name = "TYPE")]
    pub api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4)
    /// instead of relying on the auto-detection
    #[arg(long)]
    pub api_url: Option<String>,

    /// Issue description
    #[arg(short, long)]
    pub body: Option<String>,

    /// Don't open the issue in the browser after creation
    #[arg(short, long)]
    pub no_browser: bool,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    pub remote: String,

    /// Issue title
    #[arg(short, long)]
    pub title: Option<String>,

    /// Create an issue in the web browser
    #[arg(short, long)]
    pub web: bool,
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

pub struct CreateIssueOptions<'a> {
    pub title: &'a str,
    pub body: Option<&'a str>,
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

pub fn create_issue(args: IssueCreateCommandArgs) -> anyhow::Result<()> {
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if args.web {
        create_issue_via_browser(&remote, &api_type)
    } else {
        let Some(title) = args.title else {
            anyhow::bail!("--title is required when creating an issue with the CLI");
        };

        create_issue_via_api(
            &remote,
            &api_type,
            args.api_url.as_deref(),
            &CreateIssueOptions {
                title: &title,
                body: args.body.as_deref(),
            },
            args.no_browser,
        )
    }
}

// =============================================================================
// Private Helpers
// =============================================================================

fn create_issue_via_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let url = match api_type {
        ApiType::GitHub => github::get_url_for_issue_creation(remote),
        ApiType::GitLab => gitlab::get_url_for_issue_creation(remote),
        ApiType::Gitea | ApiType::Forgejo => gitea::get_url_for_issue_creation(remote),
    };

    open::that(url)?;

    Ok(())
}

fn create_issue_via_api(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    create_options: &CreateIssueOptions,
    no_browser: bool,
) -> anyhow::Result<()> {
    let http_client = HttpClient::new();
    let create_issue = match api_type {
        ApiType::GitHub => github::create_issue,
        ApiType::GitLab => gitlab::create_issue,
        ApiType::Gitea | ApiType::Forgejo => gitea::create_issue,
    };
    let issue = create_issue(&http_client, remote, api_url, create_options)?;

    if no_browser {
        println!("Issue created at {}", issue.url);
    } else {
        open::that(&issue.url)?;
    }

    Ok(())
}

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
