mod forge {
    mod forge_client;
    mod gitea;
    mod github;
    mod gitlab;
    mod http_client;

    pub use forge_client::{ApiType, ForgeClient, create_forge_client};
}

mod issue;
mod pr;
mod web;

pub use issue::list_issues;
pub use pr::{PrCommand, checkout_pr, create_pr, list_prs};
pub use web::print_web_url;

use clap::{Parser, Subcommand};

use crate::cli::{issue::IssueCommandArgs, pr::PrCommandArgs, web::WebCommandArgs};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: GitForgeCommand,
}

#[derive(Subcommand)]
pub enum GitForgeCommand {
    /// List issues from the remote repository.
    #[command(alias = "i", about = "List issues from the remote repository")]
    Issue(IssueCommandArgs),

    /// Interact with pull requests.
    #[command(alias = "p", about = "Interact with pull requests")]
    Pr(PrCommandArgs),

    /// Get the web URL for the remote repository.
    #[command(alias = "w", about = "Get the web URL for the remote repository")]
    Web(WebCommandArgs),
}
