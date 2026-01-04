//! The `issue` subcommand.

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use dialoguer::Input;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{
        forge::{self, ApiType, HttpClient, gitea, github, gitlab},
        input,
    },
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
    /// List issues.
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
    api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4)
    /// instead of relying on the auto-detection
    #[arg(long)]
    api_url: Option<String>,

    /// Use authentication with environment variables (GIT_FORGE_GITHUB_TOKEN,
    /// GIT_FORGE_GITLAB_TOKEN, GIT_FORGE_GITEA_TOKEN)
    #[arg(long)]
    auth: bool,

    /// Filter by assignee
    #[arg(long, value_name = "USERNAME")]
    assignee: Option<String>,

    #[arg(long, value_name = "USERNAME", help = "Filter by author")]
    author: Option<String>,

    /// Fields to include in output (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    fields: Vec<IssueField>,

    /// Filter by labels (comma-separated)
    #[arg(long, value_delimiter = ',')]
    labels: Vec<String>,

    /// Page number to fetch
    #[arg(long, default_value_t = 1, value_name = "NUMBER")]
    page: u32,

    /// Number of issues per page
    #[arg(long, short_alias = 'l', alias = "limit", default_value_t = DEFAULT_PER_PAGE, value_name = "NUMBER")]
    per_page: u32,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    remote: String,

    /// Filter by state
    #[arg(long)]
    state: Option<IssueState>,

    /// Open the issues page in the web browser
    #[arg(short, long)]
    web: bool,
}

/// Command-line arguments for creating an issue.
#[derive(Args)]
pub struct IssueCreateCommandArgs {
    /// Specify the forge which affects the API schema etc.
    #[arg(long, value_name = "TYPE")]
    api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4)
    /// instead of relying on the auto-detection
    #[arg(long)]
    api_url: Option<String>,

    /// Issue description
    #[arg(short, long)]
    body: Option<String>,

    /// Open your text editor to write the issue message
    #[arg(short, long)]
    editor: bool,

    /// Don't open the issue in the browser after creation
    #[arg(short, long)]
    no_browser: bool,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    remote: String,

    /// Issue title
    #[arg(short, long)]
    title: Option<String>,

    /// Create an issue in the web browser
    #[arg(short, long)]
    web: bool,
}

// =============================================================================
// Domain Types
// =============================================================================

#[derive(Clone, Debug, Deserialize, Serialize, ValueEnum)]
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

#[derive(Clone, Debug, Deserialize, Serialize, ValueEnum)]
pub enum IssueField {
    Id,
    Title,
    State,
    Labels,
    Author,
    Url,
}

/// An issue from a git forge.
#[derive(Serialize)]
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
    pub assignee: Option<&'a str>,
    pub author: Option<&'a str>,
    pub labels: &'a [String],
    pub page: u32,
    pub per_page: u32,
    pub state: &'a IssueState,
}

pub struct CreateIssueOptions<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

// =============================================================================
// Command Logic
// =============================================================================

/// Lists issues from the remote repository's forge and outputs them or
/// open the issues page in the web browser.
pub fn list_issues(args: IssueListCommandArgs) -> anyhow::Result<()> {
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if args.web {
        list_issues_in_web_browser(&remote, &api_type)
    } else {
        list_issues_to_stdout(
            &remote,
            &api_type,
            args.api_url.as_deref(),
            &ListIssueFilters {
                assignee: args.assignee.as_deref(),
                author: args.author.as_deref(),
                labels: &args.labels,
                page: args.page,
                per_page: args.per_page,
                state: &args.state.unwrap_or(IssueState::Open),
            },
            args.fields,
            args.auth,
        )
    }
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
        return create_issue_via_browser(&remote, &api_type);
    }

    if args.editor {
        return create_issue_with_text_editor(
            &remote,
            &api_type,
            args.api_url.as_deref(),
            args.no_browser,
        );
    }

    let title = match args.title {
        Some(t) => t,
        None => Input::new()
            .with_prompt("Enter issue title")
            .interact_text()?,
    };

    create_issue_via_api(
        &remote,
        &api_type,
        args.api_url.as_deref(),
        &CreateIssueOptions {
            title: &title,
            body: &args.body.unwrap_or_default(),
        },
        args.no_browser,
    )
}

// =============================================================================
// Private Helpers
// =============================================================================

fn list_issues_in_web_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let get_issues_url = match api_type {
        ApiType::GitHub => github::get_url_for_issues,
        ApiType::GitLab => gitlab::get_url_for_issues,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_issues,
    };

    open::that(get_issues_url(remote))?;

    Ok(())
}

fn list_issues_to_stdout(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    filters: &ListIssueFilters,
    fields: Vec<IssueField>,
    use_auth: bool,
) -> anyhow::Result<()> {
    let get_issues = match api_type {
        ApiType::GitHub => github::get_issues,
        ApiType::GitLab => gitlab::get_issues,
        ApiType::Gitea | ApiType::Forgejo => gitea::get_issues,
    };
    let issues = get_issues(&HttpClient::new(), remote, api_url, filters, use_auth)
        .context("Failed fetching issues")?;

    let output = format_issues_to_tsv(
        &issues,
        if fields.is_empty() {
            vec![IssueField::Id, IssueField::Title, IssueField::Url]
        } else {
            fields
        },
    );

    if !output.is_empty() {
        println!("{output}");
    }

    Ok(())
}

fn create_issue_via_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let url = match api_type {
        ApiType::GitHub => github::get_url_for_issue_creation(remote),
        ApiType::GitLab => gitlab::get_url_for_issue_creation(remote),
        ApiType::Gitea | ApiType::Forgejo => gitea::get_url_for_issue_creation(remote),
    };

    open::that(url)?;

    Ok(())
}

fn create_issue_with_text_editor(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    no_browser: bool,
) -> anyhow::Result<()> {
    let message = input::open_text_editor_to_write_message()?;

    create_issue_via_api(
        remote,
        api_type,
        api_url,
        &CreateIssueOptions {
            title: &message.title,
            body: &message.body,
        },
        no_browser,
    )
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

fn format_issues_to_tsv(issues: &[Issue], fields: Vec<IssueField>) -> String {
    issues
        .iter()
        .map(|issue| {
            fields
                .iter()
                .map(|f| get_field_value_for_issue(f, issue))
                .collect::<Vec<String>>()
                .join("\t")
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_field_value_for_issue(field: &IssueField, issue: &Issue) -> String {
    match field {
        IssueField::Id => issue.id.to_string(),
        IssueField::Title => escape_tsv(&issue.title),
        IssueField::State => issue.state.to_string(),
        IssueField::Labels => escape_tsv(&issue.labels.join(",")),
        IssueField::Author => escape_tsv(&issue.author),
        IssueField::Url => issue.url.clone(),
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
