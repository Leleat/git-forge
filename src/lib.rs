mod cli;
mod git;

use clap::Parser;

use crate::cli::{Cli, GitForgeCommand, IssueCommand, PrCommand};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        GitForgeCommand::Browse(args) => cli::browse_repository(args),
        GitForgeCommand::Completions(args) => cli::generate_completions(args),
        GitForgeCommand::Issue(args) => match args.subcommand {
            IssueCommand::List(args) => cli::list_issues(args),
        },
        GitForgeCommand::Pr(args) => match args.subcommand {
            PrCommand::Checkout(args) => cli::checkout_pr(args),
            PrCommand::Create(args) => cli::create_pr(args),
            PrCommand::List(args) => cli::list_prs(args),
        },
    }
}
