//! The `pr` subcommand.

use anyhow::Context;
use clap::{ArgAction, Args, Subcommand};

use crate::{
    cli::forge::{self, ApiType},
    git,
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
    #[command(alias = "co", about = "Checkout a pull request locally")]
    Checkout(PrCheckoutCommandArgs),

    /// Create a new pull request from the current branch.
    #[command(
        alias = "cr",
        about = "Create a new pull request from the current branch"
    )]
    Create(PrCreateCommandArgs),

    /// List pull requests as TSV.
    #[command(alias = "l", about = "List pull requests as TSV")]
    List(PrListCommandArgs),
}

/// Command-line arguments for checking out a pull request.
#[derive(Args)]
pub struct PrCheckoutCommandArgs {
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

    #[arg(help = "PR number to checkout")]
    pub number: u32,

    #[arg(long, default_value = "origin", help = "Git remote to use")]
    pub remote: String,
}

/// Command-line arguments for creating a new pull request.
#[derive(Args)]
pub struct PrCreateCommandArgs {
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

    #[arg(long, help = "PR description")]
    pub body: Option<String>,

    #[arg(long, help = "Create as draft PR")]
    pub draft: bool,

    #[arg(long, default_value = "true", action = ArgAction::Set, help = "Push branch to remote")]
    pub push: bool,

    #[arg(long, default_value = "origin", help = "Git remote to use")]
    pub remote: String,

    #[arg(long, help = "Target branch")]
    pub target: Option<String>,

    #[arg(long, help = "PR title")]
    pub title: Option<String>,
}

/// Command-line arguments for listing pull requests.
#[derive(Args)]
pub struct PrListCommandArgs {
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

    #[arg(long, help = "Filter to only draft PRs")]
    pub draft: bool,

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
        help = "Number of PRs per page"
    )]
    pub per_page: u32,

    #[arg(long, default_value = "origin", help = "Git remote to use")]
    pub remote: String,

    #[arg(long, help = "Filter by state")]
    pub state: Option<PrState>,
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

// =============================================================================
// Command Logic
// =============================================================================

/// Lists pull requests from the remote repository's forge and outputs them as
/// TSV.
pub fn list_prs(args: PrListCommandArgs) -> anyhow::Result<()> {
    let forge = forge::create_forge_client(args.remote, args.api, args.api_url)?;
    let prs = forge.get_prs(
        args.auth,
        args.author.as_deref(),
        args.labels.as_ref(),
        args.page,
        args.per_page,
        args.state.unwrap_or(PrState::Open),
        args.draft,
    )?;
    let columns = if args.columns.is_empty() {
        None
    } else {
        Some(args.columns)
    };
    let output = format_prs_to_tsv(&prs, columns);

    if !output.is_empty() {
        println!("{output}");
    }

    Ok(())
}

/// Checks out a pull request as a local branch.
pub fn checkout_pr(args: PrCheckoutCommandArgs) -> anyhow::Result<()> {
    let pr_number = args.number;
    let branch_name = format!("pr-{pr_number}");
    let remote = args.remote.clone();
    let forge = forge::create_forge_client(args.remote, args.api, args.api_url)?;
    let pr_ref = forge.get_pr_ref(pr_number);

    git::fetch_pull_request(&pr_ref, &branch_name, &remote)?;
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

    let title = args.title.unwrap_or_else(|| current_branch.clone());

    if args.push {
        git::push_branch(&current_branch, &args.remote, true)?;
    }

    let forge_client = forge::create_forge_client(args.remote, args.api, args.api_url)?;
    let pr = forge_client.create_pr(
        &title,
        &current_branch,
        &target_branch,
        args.body.as_deref(),
        args.draft,
    )?;

    println!("PR created at {}", pr.url);

    Ok(())
}

// =============================================================================
// Private Helpers
// =============================================================================

fn format_prs_to_tsv(prs: &[Pr], columns: Option<Vec<String>>) -> String {
    let columns =
        columns.unwrap_or_else(|| vec!["id".to_string(), "title".to_string(), "url".to_string()]);

    prs.iter()
        .map(|pr| {
            columns
                .iter()
                .map(|col| get_column_value_for_pr(col, pr))
                .collect::<Vec<String>>()
                .join("\t")
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_column_value_for_pr(column: &str, pr: &Pr) -> String {
    match column {
        "id" => pr.id.to_string(),
        "title" => escape_tsv(&pr.title),
        "state" => pr.state.clone(),
        "labels" => escape_tsv(&pr.labels.join(",")),
        "author" => escape_tsv(&pr.author),
        "created" => pr.created_at.clone(),
        "updated" => pr.updated_at.clone(),
        "url" => pr.url.clone(),
        "source" => pr.source_branch.clone(),
        "target" => pr.target_branch.clone(),
        "draft" => (if pr.draft { "true" } else { "false" }).to_string(),
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
