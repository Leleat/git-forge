//! The `web` subcommand.

use clap::{Args, ValueEnum};

use crate::cli::forge::{self, ApiType};

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
    let forge_client = forge::create_forge_client(args.remote, args.api, None)?;
    let url = forge_client.get_web_url(args.target.unwrap_or(WebTarget::Repository))?;

    println!("{url}");

    Ok(())
}
