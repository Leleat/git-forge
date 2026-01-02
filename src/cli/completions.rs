//! The `completions` subcommand.

use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};

use crate::cli::Cli;

// =============================================================================
// CLI Arguments
// =============================================================================

#[derive(Debug, Args)]
pub struct CompletionsCommandArgs {
    /// The shell to generate completions for
    shell: Shell,
}

// =============================================================================
// Command Logic
// =============================================================================

pub fn generate_completions(args: CompletionsCommandArgs) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    generate(args.shell, &mut cmd, bin_name, &mut std::io::stdout());

    Ok(())
}
