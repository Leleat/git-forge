mod cli;
mod git;
mod io;
mod tui;

use clap::Parser;

use crate::cli::{Cli, ConfigCommand, GitForgeCommand, IssueCommand, PrCommand};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        GitForgeCommand::Browse(args) => cli::browse_repository(args),
        GitForgeCommand::Completions(args) => cli::generate_completions(args),
        GitForgeCommand::Config(args) => match args.subcommand {
            ConfigCommand::Get(args) => cli::config_get(args),
            ConfigCommand::Set(args) => cli::config_set(args),
            ConfigCommand::Unset(args) => cli::config_unset(args),
            ConfigCommand::Edit => cli::config_edit(),
        },
        GitForgeCommand::Issue(args) => match args.subcommand {
            IssueCommand::List(args) => cli::list_issues(args),
            IssueCommand::Create(args) => cli::create_issue(args),
        },
        GitForgeCommand::Pr(args) => match args.subcommand {
            PrCommand::Checkout(args) => cli::checkout_pr(args),
            PrCommand::Create(args) => cli::create_pr(args),
            PrCommand::List(args) => cli::list_prs(args),
        },
    }
}
