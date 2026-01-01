mod forge {
    pub mod gitea;
    pub mod github;
    pub mod gitlab;

    mod api_type;
    mod http_client;

    pub use api_type::{ApiType, guess_api_type_from_host};
    pub use http_client::HttpClient;
}

mod completions;
mod issue;
mod pr;
mod web;

pub use completions::generate_completions;
pub use issue::{IssueCommand, list_issues};
pub use pr::{PrCommand, checkout_pr, create_pr, list_prs};
pub use web::print_web_url;

use clap::{Parser, Subcommand};

use crate::cli::{
    completions::CompletionsCommandArgs, issue::IssueCommandArgs, pr::PrCommandArgs,
    web::WebCommandArgs,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: GitForgeCommand,
}

#[derive(Subcommand)]
pub enum GitForgeCommand {
    /// Generate shell completions.
    #[command(alias = "c", about = "Generate shell completions")]
    Completions(CompletionsCommandArgs),

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
