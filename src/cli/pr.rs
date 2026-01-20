//! The `pr` subcommand.

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

/// Command-line arguments for the `pr` subcommand.
#[derive(Args)]
pub struct PrCommandArgs {
    #[command(subcommand)]
    pub subcommand: PrCommand,
}

/// Available subcommands for pull request operations.
#[derive(Subcommand)]
pub enum PrCommand {
    /// Checkout a pull request locally.
    #[command(alias = "co")]
    Checkout(PrCheckoutCommandArgs),

    /// Create a new pull request from the current branch and open the pull
    /// request in the web browser.
    #[command(alias = "cr")]
    Create(PrCreateCommandArgs),

    /// List pull requests.
    #[command(alias = "ls")]
    List(PrListCommandArgs),
}

/// Command-line arguments for checking out a pull request.
#[derive(Args)]
pub struct PrCheckoutCommandArgs {
    /// Specify the forge which affects the API schema etc
    #[arg(long, value_name = "TYPE")]
    api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4)
    /// instead of relying on the auto-detection
    #[arg(long)]
    api_url: Option<String>,

    /// Use authentication with environment variables (GIT_FORGE_GITHUB_TOKEN,
    /// GIT_FORGE_GITLAB_TOKEN, GIT_FORGE_GITEA_TOKEN) for interactive selection
    #[arg(long)]
    auth: bool,

    /// Filter by author username for interactive selection
    #[arg(long)]
    author: Option<String>,

    /// Filter to only draft PRs for interactive selection
    #[arg(long)]
    draft: bool,

    /// Filter by labels (comma-separated) for interactive selection
    #[arg(long, value_delimiter = ',')]
    labels: Vec<String>,

    /// PR number to checkout. Omit for interactive selection
    number: Option<u32>,

    /// Number of PRs per page for interactive selection
    #[arg(long, short_alias = 'l', alias = "limit", value_name = "NUMBER")]
    per_page: Option<u32>,

    /// Search keywords
    #[arg(short, long)]
    query: Option<String>,

    /// Git remote to use
    #[arg(long)]
    remote: Option<String>,

    /// Filter by state for interactive selection
    #[arg(long)]
    state: Option<PrState>,
}

/// Command-line arguments for creating a new pull request.
#[derive(Args)]
pub struct PrCreateCommandArgs {
    /// Specify the forge which affects the API schema etc
    #[arg(long, value_name = "TYPE")]
    api: Option<ApiType>,

    /// Explicitly provide the base API URL (e.g. https://gitlab.com/api/v4) instead of relying on the auto-detection
    #[arg(long)]
    api_url: Option<String>,

    // PR description
    #[arg(long)]
    body: Option<String>,

    /// Create as draft PR
    #[arg(long)]
    draft: bool,

    /// Open your text editor to write the pr message
    #[arg(short, long, group = "input-mode")]
    editor: bool,

    /// Use the branch name as the PR title and put the commit subjects in a
    /// list as the PR description
    #[arg(short, long, group = "input-mode")]
    fill: bool,

    /// Use the first commit subject as the PR title and put the first commit
    /// body as the PR description
    #[arg(long, group = "input-mode")]
    fill_first: bool,

    /// Use the branch name as the PR title and put all commit messages in a
    /// list (subject and body) as the PR description
    #[arg(long, group = "input-mode")]
    fill_verbose: bool,

    /// Don't open the issue in the browser after creation
    #[arg(short, long)]
    no_browser: bool,

    /// Don't push the branch. Expect the branch to already exist at the remote.
    #[arg(long)]
    no_push: bool,

    /// Git remote to use
    #[arg(long)]
    remote: Option<String>,

    /// Target branch
    #[arg(long)]
    target: Option<String>,

    /// PR title
    #[arg(long)]
    title: Option<String>,

    /// Create a PR in the web browser
    #[arg(short, long)]
    web: bool,
}

/// Command-line arguments for listing pull requests.
#[derive(Args, Clone)]
pub struct PrListCommandArgs {
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

    /// Filter by author username
    #[arg(long)]
    author: Option<String>,

    /// Filter to only draft PRs
    #[arg(long)]
    draft: bool,

    /// Fields to include in output (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    fields: Vec<PrField>,

    /// Output format
    #[arg(short = 'o', long)]
    format: Option<OutputFormat>,

    /// Use interactive TUI for searching and selecting a PR
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

    /// Number of PRs per page
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
    state: Option<PrState>,

    /// Open the prs page in the web browser
    #[arg(short, long)]
    web: bool,
}

// =============================================================================
// Domain Types
// =============================================================================

#[derive(Clone, Default, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    /// Open pull requests that haven't been closed or merged.
    #[default]
    Open,
    /// Closed pull requests that were not merged.
    Closed,
    /// Pull requests that have been merged.
    Merged,
    /// All pull requests regardless of state.
    All,
}

impl std::fmt::Display for PrState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrState::Open => write!(f, "open"),
            PrState::Closed => write!(f, "closed"),
            PrState::Merged => write!(f, "merged"),
            PrState::All => write!(f, "all"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum PrField {
    Id,
    Title,
    State,
    Labels,
    Author,
    CreatedAt,
    UpdatedAt,
    Url,
    Source,
    Target,
    Draft,
}

#[derive(Clone, Serialize)]
pub struct Pr {
    /// The pull request number (e.g., #42).
    pub id: u32,
    /// The pull request title.
    pub title: String,
    /// The current state (open, closed, merged).
    pub state: String,
    /// The username of the pull request author.
    pub author: String,
    /// The web URL to view this pull request.
    pub url: String,
    /// Labels attached to this pull request.
    pub labels: Vec<String>,
    /// Timestamp when the pull request was created.
    pub created_at: String,
    /// Timestamp when the pull request was last updated.
    pub updated_at: String,
    /// Whether the pull request is a draft.
    pub draft: bool,
}

impl ListableItem for Pr {
    fn get_display_text(&self) -> String {
        format!("{}: {}", self.id, self.title)
    }
}

pub struct ListPrsFilters<'a> {
    pub author: Option<&'a str>,
    pub labels: &'a [String],
    pub page: u32,
    pub per_page: u32,
    pub query: Option<&'a str>,
    pub state: &'a PrState,
    pub draft: bool,
}

pub struct CreatePrOptions<'a> {
    pub title: &'a str,
    pub source_branch: &'a str,
    pub target_branch: &'a str,
    pub body: &'a str,
    pub draft: bool,
}

// =============================================================================
// Command Logic
// =============================================================================

/// Lists pull requests from the remote repository's forge and outputs them as
/// TSV or open the prs page in the web browser.
pub fn list_prs(mut args: PrListCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("pr/list/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote = git::get_remote_data(&remote_name)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &remote_name))?;

    config::merge_config_into_args!(
        &config,
        args,
        Some(&remote),
        "pr/list",
        [
            api,
            api_url,
            auth,
            draft,
            fields,
            format,
            interactive,
            per_page,
            state
        ]
    );

    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if args.interactive {
        list_prs_interactively(remote, api_type, args)
    } else if args.web {
        list_prs_in_web_browser(&remote, &api_type)
    } else {
        list_prs_to_stdout(
            &remote,
            &api_type,
            args.api_url.as_deref(),
            &ListPrsFilters {
                author: args.author.as_deref(),
                labels: &args.labels,
                page: args.page,
                per_page: args.per_page.unwrap_or(DEFAULT_PER_PAGE),
                query: args.query.as_deref(),
                state: &args.state.unwrap_or_default(),
                draft: args.draft,
            },
            args.fields,
            &args.format.unwrap_or_default(),
            args.auth,
        )
    }
}

/// Checks out a pull request as a local branch.
pub fn checkout_pr(mut args: PrCheckoutCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("pr/checkout/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote_result = git::get_remote_data(&remote_name);

    config::merge_config_into_args!(
        &config,
        args,
        remote_result.as_ref().ok(),
        "pr/checkout",
        [api, api_url, auth, author, draft, per_page, state]
    );

    // Allow remote detection to fail if user provides --api explicitly.
    let api_type = match remote_result {
        Ok(ref remote) => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
        Err(ref e) => match args.api {
            Some(api_type) => api_type,
            None => anyhow::bail!(
                "Could not detect the forge type from the remote URL: {e}\nSpecify the forge type explicitly with --api (github|gitlab|gitea|forgejo)"
            ),
        },
    };
    let get_pr_ref = forge::function!(api_type, get_pr_ref);
    let pr_number = match args.number {
        Some(nr) => nr,
        None => {
            let remote = remote_result?;
            let fetch_options = tui::build_fetch_options! {
                "author": args.author,
                "draft": args.draft,
                "labels": args.labels,
                "query": args.query,
                "state": args.state,
            };

            eprintln!("Loading pull requests...");

            let pr = select_pr_interactively(
                remote,
                api_type,
                args.api_url,
                fetch_options,
                args.per_page.unwrap_or(DEFAULT_PER_PAGE),
                args.auth,
            )?;

            pr.id
        }
    };
    let pr_ref = get_pr_ref(pr_number);
    let branch_name = format!("pr-{pr_number}");

    eprintln!("Fetching {pr_ref}:{branch_name} from {remote_name}...");
    git::fetch_pull_request(&pr_ref, &branch_name, &remote_name)?;

    eprintln!("Checking out {branch_name}...");
    git::checkout_branch(&branch_name)?;

    eprintln!("Successfully checked out PR \"{pr_number}\" to branch \"{branch_name}\"");

    Ok(())
}

/// Creates a new pull request from the current branch.
pub fn create_pr(mut args: PrCreateCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("pr/create/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote = git::get_remote_data(&remote_name)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &remote_name))?;

    config::merge_config_into_args!(
        &config,
        args,
        Some(&remote),
        "pr/create",
        [
            api,
            api_url,
            draft,
            editor,
            fill,
            fill_first,
            fill_verbose,
            no_browser,
            no_push,
            target
        ]
    );

    let current_branch = git::get_current_branch()?;
    let target_branch = match args.target {
        Some(target) => target,
        None => git::get_default_branch(&remote_name)
            .context("Could not determine the target branch for this PR")?,
    };

    if current_branch == target_branch {
        anyhow::bail!(
            "Cannot create PR: current branch \"{}\" is the same as target branch.",
            current_branch
        );
    }

    let http_client = HttpClient::new();
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if !args.no_push {
        eprintln!("Pushing branch '{current_branch}'...");

        git::push_branch(&current_branch, &remote_name, true)?;
    }

    if args.web {
        return create_pr_in_browser(&api_type, &remote, &target_branch, &current_branch);
    }

    let create_pr = forge::function!(api_type, create_pr);

    let (title, body) = if args.editor {
        get_title_and_body_for_pr_for_editor_flag(
            config
                .get_string_from_global_scope("editor-command")
                .as_deref(),
        )?
    } else if args.fill {
        let (generated_title, generated_body) =
            get_title_and_body_for_pr_for_fill_flag(&target_branch, &current_branch)?;

        (
            args.title.unwrap_or(generated_title),
            args.body.unwrap_or(generated_body),
        )
    } else if args.fill_first {
        let (generated_title, generated_body) =
            get_title_and_body_for_pr_for_fill_first_flag(&target_branch, &current_branch)?;

        (
            args.title.unwrap_or(generated_title),
            args.body.unwrap_or(generated_body),
        )
    } else if args.fill_verbose {
        let (generated_title, generated_body) =
            get_title_and_body_for_pr_for_fill_verbose_flag(&target_branch, &current_branch)?;

        (
            args.title.unwrap_or(generated_title),
            args.body.unwrap_or(generated_body),
        )
    } else {
        (
            match args.title {
                Some(t) => t,
                None => Input::new().with_prompt("Enter PR title").interact_text()?,
            },
            args.body.unwrap_or_default(),
        )
    };

    let create_options = CreatePrOptions {
        title: &title,
        source_branch: &current_branch,
        target_branch: &target_branch,
        body: &body,
        draft: args.draft,
    };
    let pr = create_pr(
        &http_client,
        &remote,
        args.api_url.as_deref(),
        &create_options,
    )?;

    if args.no_browser {
        println!("{}", pr.url);
    } else {
        eprintln!("Opening PR in browser: {}", pr.url);

        open::that(pr.url)?;
    }

    Ok(())
}

// =============================================================================
// Private Helpers
// =============================================================================

fn create_pr_in_browser(
    api_type: &ApiType,
    remote: &GitRemoteData,
    target_branch: &str,
    source_branch: &str,
) -> anyhow::Result<()> {
    let get_url_for_creation = forge::function!(api_type, get_url_for_pr_creation);

    let url = get_url_for_creation(remote, target_branch, source_branch);

    eprintln!(
        "Opening URL to create PR for {source_branch} targeting {target_branch} in browser..."
    );

    open::that(&url)?;

    eprintln!("Opened URL in browser: {url}");

    Ok(())
}

fn get_title_and_body_for_pr_for_editor_flag(
    editor_command: Option<&str>,
) -> anyhow::Result<(String, String)> {
    let message = match editor_command {
        Some(cmd) => io::prompt_with_custom_text_editor(cmd),
        None => io::prompt_with_default_text_editor(),
    }?;

    if message.title.is_empty() {
        anyhow::bail!("PR title cannot be empty. Please provide a title on the first line.");
    }

    Ok((message.title, message.body))
}

fn get_title_and_body_for_pr_for_fill_flag(
    target_branch: &str,
    current_branch: &str,
) -> anyhow::Result<(String, String)> {
    let commit_shas = git::get_commit_range(target_branch, current_branch)
        .context("Failed to get commits for PR")?;

    if commit_shas.is_empty() {
        anyhow::bail!(
            "No commits found between '{target_branch}' and '{current_branch}'. Cannot create PR with --fill.",
        );
    }

    let title = current_branch.to_string();
    let body = commit_shas
        .iter()
        .rev()
        .map(|sha| {
            git::get_commit_message(sha)
                .context("Failed to get commit message")
                .map(|(subject, _)| format!("- {subject}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");

    Ok((title, body))
}

fn get_title_and_body_for_pr_for_fill_first_flag(
    target_branch: &str,
    current_branch: &str,
) -> anyhow::Result<(String, String)> {
    let commit_shas = git::get_commit_range(target_branch, current_branch)
        .context("Failed to get commits for PR")?;

    if let Some(last_commit) = commit_shas.last() {
        return git::get_commit_message(last_commit).context("Failed to get commit message");
    }

    anyhow::bail!(
        "No commits found between '{target_branch}' and '{current_branch}'. Cannot create PR with --fill-first.",
    );
}

fn get_title_and_body_for_pr_for_fill_verbose_flag(
    target_branch: &str,
    current_branch: &str,
) -> anyhow::Result<(String, String)> {
    let commit_shas = git::get_commit_range(target_branch, current_branch)
        .context("Failed to get commits for PR")?;

    if commit_shas.is_empty() {
        anyhow::bail!(
            "No commits found between '{target_branch}' and '{current_branch}'. Cannot create PR with --fill-verbose.",
        );
    }

    let title = current_branch.to_string();
    let body = commit_shas
        .iter()
        .rev()
        .map(|sha| {
            git::get_commit_message(sha)
                .context("Failed to get commit message")
                .map(|(subject, body)| {
                    if body.is_empty() {
                        format!("- **{subject}**")
                    } else {
                        let body = body
                            .split("\n")
                            .map(|line| format!("  {line}"))
                            .collect::<Vec<String>>()
                            .join("\n");

                        format!("- **{subject}**\n{body}")
                    }
                })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");

    Ok((title, body))
}

fn list_prs_in_web_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let get_prs_url = forge::function!(api_type, get_url_for_prs);

    open::that(get_prs_url(remote))?;

    Ok(())
}

fn list_prs_to_stdout(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    filters: &ListPrsFilters,
    fields: Vec<PrField>,
    output_format: &OutputFormat,
    use_auth: bool,
) -> anyhow::Result<()> {
    let get_prs = forge::function!(api_type, get_prs);
    let response = get_prs(&HttpClient::new(), remote, api_url, filters, use_auth)?;

    let fields = if fields.is_empty() {
        vec![PrField::Title, PrField::Id, PrField::Url]
    } else {
        fields
    };

    if !response.items.is_empty() {
        println!("{}", io::format(&response.items, &fields, output_format)?);
    }

    Ok(())
}

fn list_prs_interactively(
    remote: GitRemoteData,
    api_type: ApiType,
    args: PrListCommandArgs,
) -> anyhow::Result<()> {
    let fetch_options = tui::build_fetch_options!(
        "author": args.author,
        "draft": args.draft,
        "labels": args.labels,
        "query": args.query,
        "state": args.state,
    );

    eprintln!("Loading pull requests...");

    let pr = select_pr_interactively(
        remote,
        api_type,
        args.api_url,
        fetch_options,
        args.per_page.unwrap_or(DEFAULT_PER_PAGE),
        args.auth,
    )?;

    let output_format = args.format.unwrap_or_default();
    let fields = if args.fields.is_empty() {
        vec![PrField::Title, PrField::Id, PrField::Url]
    } else {
        args.fields
    };

    println!("{}", io::format(&[&pr], &fields, &output_format)?);

    if args.web {
        open::that(pr.url)?;
    }

    Ok(())
}

fn select_pr_interactively(
    remote: GitRemoteData,
    api_type: ApiType,
    api_url: Option<String>,
    initial_options: FetchOptions,
    per_page: u32,
    use_auth: bool,
) -> anyhow::Result<Pr> {
    let get_prs = forge::function!(api_type, get_prs);

    let http_client = HttpClient::new();

    tui::select_item_with(initial_options, move |page, options, result| {
        let author: Option<&str> = options.parse_str("author");
        let draft: bool = options.parse("draft").unwrap_or_default();
        let labels: Vec<String> = options.parse_list("labels").unwrap_or_default();
        let query: Option<&str> = options.parse_str("query");
        let state: PrState = options.parse_enum("state").unwrap_or_default();

        let response = get_prs(
            &http_client,
            &remote,
            api_url.as_deref(),
            &ListPrsFilters {
                author,
                draft,
                labels: &labels,
                page,
                per_page,
                query,
                state: &state,
            },
            use_auth,
        )?;

        Ok(result
            .with_items(response.items)
            .with_more_items(response.has_next_page))
    })
}
