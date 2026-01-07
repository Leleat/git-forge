mod forge {
    pub mod gitea;
    pub mod github;
    pub mod gitlab;

    mod api_type;
    mod http_client;

    pub use api_type::{ApiType, guess_api_type_from_host};
    pub use http_client::HttpClient;
}

mod browse;
mod completions;
mod config;
mod issue;
mod pr;

pub use browse::browse_repository;
pub use completions::generate_completions;
pub use config::{ConfigCommand, config_edit, config_get, config_set, config_unset};
pub use issue::{IssueCommand, create_issue, list_issues};
pub use pr::{PrCommand, checkout_pr, create_pr, list_prs};

use clap::{Parser, Subcommand};

use crate::cli::{
    browse::BrowseCommandArgs, completions::CompletionsCommandArgs, config::ConfigCommandArgs,
    issue::IssueCommandArgs, pr::PrCommandArgs,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: GitForgeCommand,
}

#[derive(Subcommand)]
pub enum GitForgeCommand {
    /// Open repository links in your browser or print them to stdout
    #[command(alias = "b")]
    Browse(BrowseCommandArgs),

    /// Generate shell completions.
    #[command(alias = "c")]
    Completions(CompletionsCommandArgs),

    #[command(about = "
Manage configuration settings.

Currently supported settings:

  - editor-command: This command will be called instead of the default text editor when using the --editor flag. E.g. for vscode use `code --wait`
  - <CLI_OPTIONS>: Most CLI options can be configured with a scoped default setting. See the config subcommands' help for more details.
    ")]
    Config(ConfigCommandArgs),

    /// List issues from the remote repository.
    #[command(alias = "i")]
    Issue(IssueCommandArgs),

    /// Interact with pull requests.
    #[command(alias = "p")]
    Pr(PrCommandArgs),
}
