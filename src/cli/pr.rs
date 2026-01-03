//! The `pr` subcommand.

use anyhow::Context;
use clap::{Args, Subcommand};
use dialoguer::Input;

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

    /// PR number to checkout
    number: u32,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    remote: String,
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
    #[arg(short, long)]
    editor: bool,

    /// Don't open the issue in the browser after creation
    #[arg(short, long)]
    no_browser: bool,

    /// Don't push the branch. Expect the branch to already exist at the remote.
    #[arg(long)]
    no_push: bool,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    remote: String,

    /// Target branch
    #[arg(long)]
    target: Option<String>,

    /// PR title
    #[arg(long)]
    title: Option<String>,
}

/// Command-line arguments for listing pull requests.
#[derive(Args)]
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
    #[arg(long, value_delimiter = ',')]
    fields: Vec<PrField>,

    /// Filter by labels (comma-separated)
    #[arg(long, value_delimiter = ',')]
    labels: Vec<String>,

    /// Page number to fetch
    #[arg(long, default_value_t = 1, value_name = "NUMBER")]
    page: u32,

    /// Number of PRs per page
    #[arg(long, short_alias = 'l', alias = "limit", default_value_t = DEFAULT_PER_PAGE, value_name = "NUMBER")]
    per_page: u32,

    /// Git remote to use
    #[arg(long, default_value = "origin")]
    remote: String,

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

#[derive(Clone, clap::ValueEnum, serde::Serialize)]
#[value(rename_all = "lower")]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    /// Open pull requests that haven't been closed or merged.
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum PrField {
    Id,
    Title,
    State,
    Labels,
    Author,
    Created,
    Updated,
    Url,
    Source,
    Target,
    Draft,
}

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
    /// The source branch (head) of the pull request.
    pub source_branch: String,
    /// The target branch (base) of the pull request.
    pub target_branch: String,
    /// Whether the pull request is a draft.
    pub draft: bool,
}

pub struct ListPrsFilters<'a> {
    pub author: Option<&'a str>,
    pub labels: &'a [String],
    pub page: u32,
    pub per_page: u32,
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
pub fn list_prs(args: PrListCommandArgs) -> anyhow::Result<()> {
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if args.web {
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
                per_page: args.per_page,
                state: &args.state.unwrap_or(PrState::Open),
                draft: args.draft,
            },
            args.fields,
            args.auth,
        )
    }
}

/// Checks out a pull request as a local branch.
pub fn checkout_pr(args: PrCheckoutCommandArgs) -> anyhow::Result<()> {
    let api_type = match git::get_remote_data(&args.remote) {
        Ok(remote) => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
        Err(e) => match args.api {
            Some(api_type) => api_type,
            None => anyhow::bail!(
                "No API type was provided and failed to guess it from the git remote URL: {}",
                e
            ),
        },
    };
    let get_pr_ref = match api_type {
        ApiType::GitHub => github::get_pr_ref,
        ApiType::GitLab => gitlab::get_pr_ref,
        ApiType::Gitea | ApiType::Forgejo => gitea::get_pr_ref,
    };
    let pr_number = args.number;
    let pr_ref = get_pr_ref(pr_number);
    let branch_name = format!("pr-{pr_number}");

    git::fetch_pull_request(&pr_ref, &branch_name, &args.remote)?;
    git::checkout_branch(&branch_name)?;

    eprintln!("Successfully checked out PR \"{pr_number}\" to branch \"{branch_name}\"");

    Ok(())
}

/// Creates a new pull request from the current branch.
pub fn create_pr(args: PrCreateCommandArgs) -> anyhow::Result<()> {
    let current_branch = git::get_current_branch()?;
    let target_branch = match args.target {
        Some(target) => target,
        None => git::get_default_branch(&args.remote)
            .context("Couldn't create a PR. You can provide a --target explicitly.")?,
    };

    if current_branch == target_branch {
        anyhow::bail!(
            "Cannot create PR: current branch \"{}\" is the same as target branch.",
            current_branch
        );
    }

    let http_client = HttpClient::new();
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if !args.no_push {
        eprintln!("Pushing branch '{current_branch}'...");

        git::push_branch(&current_branch, &args.remote, true)?;
    }

    let create_pr = match api_type {
        ApiType::GitHub => github::create_pr,
        ApiType::GitLab => gitlab::create_pr,
        ApiType::Gitea | ApiType::Forgejo => gitea::create_pr,
    };

    let (title, body) = if args.editor {
        let message = input::open_text_editor_to_write_message()?;

        (message.title, message.body)
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
        println!("PR created at {}", pr.url);
    } else {
        open::that(pr.url)?;
    }

    Ok(())
}

// =============================================================================
// Private Helpers
// =============================================================================

fn list_prs_in_web_browser(remote: &GitRemoteData, api_type: &ApiType) -> anyhow::Result<()> {
    let get_prs_url = match api_type {
        ApiType::GitHub => github::get_url_for_prs,
        ApiType::GitLab => gitlab::get_url_for_prs,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_prs,
    };

    open::that(get_prs_url(remote))?;

    Ok(())
}

fn list_prs_to_stdout(
    remote: &GitRemoteData,
    api_type: &ApiType,
    api_url: Option<&str>,
    filters: &ListPrsFilters,
    fields: Vec<PrField>,
    use_auth: bool,
) -> anyhow::Result<()> {
    let get_prs = match api_type {
        ApiType::GitHub => github::get_prs,
        ApiType::GitLab => gitlab::get_prs,
        ApiType::Gitea | ApiType::Forgejo => gitea::get_prs,
    };
    let prs = get_prs(&HttpClient::new(), remote, api_url, filters, use_auth)?;

    let output = format_prs_to_tsv(
        &prs,
        if fields.is_empty() {
            vec![PrField::Id, PrField::Title, PrField::Url]
        } else {
            fields
        },
    );

    if !output.is_empty() {
        println!("{output}");
    }

    Ok(())
}

fn format_prs_to_tsv(prs: &[Pr], fields: Vec<PrField>) -> String {
    prs.iter()
        .map(|pr| {
            fields
                .iter()
                .map(|f| get_field_value_for_pr(f, pr))
                .collect::<Vec<String>>()
                .join("\t")
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_field_value_for_pr(field: &PrField, pr: &Pr) -> String {
    match field {
        PrField::Id => pr.id.to_string(),
        PrField::Title => escape_tsv(&pr.title),
        PrField::State => pr.state.clone(),
        PrField::Labels => escape_tsv(&pr.labels.join(",")),
        PrField::Author => escape_tsv(&pr.author),
        PrField::Created => pr.created_at.clone(),
        PrField::Updated => pr.updated_at.clone(),
        PrField::Url => pr.url.clone(),
        PrField::Source => pr.source_branch.clone(),
        PrField::Target => pr.target_branch.clone(),
        PrField::Draft => (if pr.draft { "true" } else { "false" }).to_string(),
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
