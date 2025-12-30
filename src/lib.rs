mod cli;
mod git;

use clap::Parser;

use crate::cli::{Cli, GitForgeCommand, PrCommand};

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.subcommand {
        GitForgeCommand::Issue(args) => cli::list_issues(args),
        GitForgeCommand::Pr(args) => match args.subcommand {
            PrCommand::Checkout(args) => cli::checkout_pr(args),
            PrCommand::Create(args) => cli::create_pr(args),
            PrCommand::List(args) => cli::list_prs(args),
        },
        GitForgeCommand::Web(args) => cli::print_web_url(args),
    }
}
