//! The `web` subcommand.

use anyhow::Context;
use clap::{Args, ValueEnum};

use crate::{
    cli::forge::{self, ApiType, gitea, github, gitlab},
    git,
};

// =============================================================================
// CLI Arguments
// =============================================================================

/// Command-line arguments for the `web` subcommand.
#[derive(Args)]
pub struct WebCommandArgs {
    #[arg(
        long,
        value_name = "TYPE",
        help = "Specify the forge which affects the API schema etc."
    )]
    pub api: Option<ApiType>,

    #[arg(long, help = "Target URL")]
    pub target: Option<WebTarget>,

    #[arg(long, default_value = "origin", help = "Git remote to use")]
    pub remote: String,
}

// =============================================================================
// Domain Types
// =============================================================================

/// Target of the forge to generate a URL for.
#[derive(Clone, Copy, ValueEnum)]
pub enum WebTarget {
    /// The main repository page.
    Repository,
    /// The issues list page.
    Issues,
    /// The pull requests list page.
    Prs,
    /// Alias for Prs.
    Mrs,
}

// =============================================================================
// Command Logic
// =============================================================================

/// Generates and prints the web URL for the specified target.
///
/// Constructs a URL for viewing the repository, issues, or pull requests in a
/// web browser. If no target is specified, defaults to the repository page.
pub fn print_web_url(args: WebCommandArgs) -> anyhow::Result<()> {
    let remote = git::get_remote_data(&args.remote)
        .with_context(|| format!("Failed to parse remote URL for remote '{}'", &args.remote))?;
    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };
    let target = args.target.unwrap_or(WebTarget::Repository);
    let get_web_url = match api_type {
        ApiType::GitHub => github::build_web_url,
        ApiType::GitLab => gitlab::build_web_url,
        ApiType::Forgejo | ApiType::Gitea => gitea::build_web_url,
    };
    let url = get_web_url(&remote, &target);

    println!("{url}",);

    Ok(())
}
