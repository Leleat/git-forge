//! The `issue` subcommand.

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use dialoguer::Input;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{
        config::{self, Config},
        forge::{self, ApiType, HttpClient, gitea, github, gitlab},
    },
    git::{self, GitRemoteData},
    io::{self, OutputFormat},
    tui::{self, FetchOptions, ListableItem},
};

// =============================================================================
// CLI Arguments
// =============================================================================

const DEFAULT_PER_PAGE: u32 = 30;
const DEFAULT_REMOTE: &str = "origin";

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

    /// Output format
    #[arg(long)]
    format: Option<OutputFormat>,

    /// Use interactive TUI for searching and selecting an issue
    #[arg(short, long, group = "interaction-type")]
    interactive: bool,

    /// Filter by labels (comma-separated)
    #[arg(long, value_delimiter = ',')]
    labels: Vec<String>,

    /// Page number to fetch
    #[arg(
        long,
        default_value_t = 1,
        group = "interaction-type",
        value_name = "NUMBER"
    )]
    page: u32,

    /// Number of issues per page
    #[arg(long, short_alias = 'l', alias = "limit", value_name = "NUMBER")]
    per_page: Option<u32>,

    /// Search keywords
    #[arg(short, long)]
    query: Option<String>,

    /// Git remote to use
    #[arg(long)]
    remote: Option<String>,

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
    #[arg(long)]
    remote: Option<String>,

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

#[derive(Clone, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    /// Open issues that haven't been closed yet.
    #[default]
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
#[serde(rename_all = "snake_case")]
pub enum IssueField {
    Id,
    Title,
    State,
    Labels,
    Author,
    Url,
}

/// An issue from a git forge.
#[derive(Clone, Serialize)]
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

impl ListableItem for Issue {
    fn get_display_text(&self) -> String {
        format!("{}: {}", self.id, self.title)
    }
}

pub struct ListIssueFilters<'a> {
    pub assignee: Option<&'a str>,
    pub author: Option<&'a str>,
    pub labels: &'a [String],
    pub page: u32,
    pub per_page: u32,
    pub query: Option<&'a str>,
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
pub fn list_issues(mut args: IssueListCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("issue/list/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote = git::get_remote_data(&remote_name)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &remote_name))?;

    config::merge_config_into_args!(
        &config,
        args,
        Some(&remote),
        "issue/list",
        [
            api,
            api_url,
            auth,
            fields,
            format,
            per_page,
            state,
            interactive
        ]
    );

    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if args.interactive {
        list_issues_interactively(remote, api_type, args)
    } else if args.web {
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
                per_page: args.per_page.unwrap_or(DEFAULT_PER_PAGE),
                query: args.query.as_deref(),
                state: &args.state.unwrap_or_default(),
            },
            args.fields,
            &args.format.unwrap_or_default(),
            args.auth,
        )
    }
}

/// Executes the `issue create` subcommand to create an issue.
pub fn create_issue(mut args: IssueCreateCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("issue/create/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote = git::get_remote_data(&remote_name)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &remote_name))?;

    config::merge_config_into_args!(
        &config,
        args,
        Some(&remote),
        "issue/create",
        [api, api_url, editor, no_browser, web]
    );

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
            config
                .get_string_from_global_scope("editor-command")
                .as_deref(),
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
    let get_issues_url = forge::function!(api_type, get_url_for_issues);

    open::that(get_issues_url(remote))?;

    Ok(())
}

fn list_issues_to_stdout(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    filters: &ListIssueFilters,
    fields: Vec<IssueField>,
    output_format: &OutputFormat,
    use_auth: bool,
) -> anyhow::Result<()> {
    let get_issues = forge::function!(api_type, get_issues);
    let response = get_issues(&HttpClient::new(), remote, api_url, filters, use_auth)
        .context("Failed fetching issues")?;

    let fields = if fields.is_empty() {
        vec![IssueField::Title, IssueField::Id, IssueField::Url]
    } else {
        fields
    };

    if !response.items.is_empty() {
        println!("{}", io::format(&response.items, &fields, output_format)?);
    }

    Ok(())
}

fn list_issues_interactively(
    remote: GitRemoteData,
    api_type: ApiType,
    args: IssueListCommandArgs,
) -> anyhow::Result<()> {
    let fetch_options = tui::build_fetch_options! {
        "assignee": args.assignee,
        "author": args.author,
        "labels": args.labels,
        "query": args.query,
        "state": args.state,
    };

    eprintln!("Loading issues...");

    let issue = select_issue_interactively(
        remote,
        api_type,
        args.api_url,
        fetch_options,
        args.per_page.unwrap_or(DEFAULT_PER_PAGE),
        args.auth,
    )?;

    let output_format = args.format.unwrap_or_default();
    let fields = if args.fields.is_empty() {
        vec![IssueField::Title, IssueField::Id, IssueField::Url]
    } else {
        args.fields
    };

    println!("{}", io::format(&[&issue], &fields, &output_format)?);

    if args.web {
        open::that(issue.url)?;
    }

    Ok(())
}

fn select_issue_interactively(
    remote: GitRemoteData,
    api_type: ApiType,
    api_url: Option<String>,
    initial_options: FetchOptions,
    per_page: u32,
    use_auth: bool,
) -> anyhow::Result<Issue> {
    let get_issues = forge::function!(api_type, get_issues);
    let http_client = HttpClient::new();

    tui::select_item_with(initial_options, move |page, options, result| {
        let assignee = options.parse_str("assignee");
        let author = options.parse_str("author");
        let labels = options.parse_list("labels").unwrap_or_default();
        let issue_state = options.parse_enum("state").unwrap_or_default();
        let query = options.parse_str("query");

        let response = get_issues(
            &http_client,
            &remote,
            api_url.as_deref(),
            &ListIssueFilters {
                author,
                labels: &labels,
                page,
                per_page,
                query,
                state: &issue_state,
                assignee,
            },
            use_auth,
        )?;

        Ok(result
            .with_items(response.items)
            .with_more_items(response.has_next_page))
    })
}

fn create_issue_via_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let url = forge::function!(api_type, get_url_for_issue_creation)(remote);

    open::that(url)?;

    Ok(())
}

fn create_issue_with_text_editor(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    editor_command: Option<&str>,
    no_browser: bool,
) -> anyhow::Result<()> {
    let message = match editor_command {
        Some(cmd) => io::prompt_with_custom_text_editor(cmd),
        None => io::prompt_with_default_text_editor(),
    }?;

    if message.title.is_empty() {
        anyhow::bail!("Issue title cannot be empty. Please provide a title on the first line.");
    }

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
    let create_issue = forge::function!(api_type, create_issue);
    let issue = create_issue(&http_client, remote, api_url, create_options)?;

    if no_browser {
        println!("{}", issue.url);
    } else {
        eprintln!("Opening issue in browser: {}", issue.url);

        open::that(&issue.url)?;
    }

    Ok(())
}
