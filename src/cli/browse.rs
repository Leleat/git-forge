//! The `browse` subcommand.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Args;

use crate::{
    cli::{
        config::{Config, MergableWithConfig},
        forge::{self, ApiType, gitea, github, gitlab},
    },
    git::{self, GitRemoteData},
};

const DEFAULT_REMOTE: &str = "origin";

// =============================================================================
// CLI Arguments
// =============================================================================

/// Command-line arguments for the `browse` subcommand.
#[derive(Args, Debug)]
pub struct BrowseCommandArgs {
    /// Specify the forge which affects the API schema etc.
    #[arg(long, value_name = "TYPE")]
    api: Option<ApiType>,

    /// Open this commit-ish. If <PATH> is provided, open the file at this
    /// commit-ish
    #[arg(short, long, group = "input-type", value_name = "COMMIT_ISH")]
    commit: Option<String>,

    /// Open the issues page. If <NUMBER> is provided, open that specific issue
    #[arg(short, long, group = "input-type", value_name = "NUMBER")]
    issues: Option<Option<u32>>,

    /// Instead of opening the URL in your browser, print it to stdout
    #[arg(short, long)]
    no_browser: bool,

    /// The file or directory to open
    #[arg(name = "<PATH[:<LINE_NUMBER>]>")]
    path: Option<String>,

    /// Open the PR page. If <NUMBER> is provided, open that specific pr
    #[arg(
        short,
        long,
        group = "input-type",
        alias = "mrs",
        short_alias = 'm',
        value_name = "NUMBER"
    )]
    prs: Option<Option<u32>>,

    /// Git remote to use
    #[arg(long)]
    remote: Option<String>,
}

impl MergableWithConfig for BrowseCommandArgs {
    fn merge_with_config(&mut self, config: &Config, remote: Option<&GitRemoteData>) {
        if self.api.is_none() {
            self.api = config.get_enum("browse/api", remote);
        }

        if !self.no_browser {
            self.no_browser = config
                .get_bool("browse/no-browser", remote)
                .unwrap_or_default();
        }
    }
}

// =============================================================================
// Command Logic
// =============================================================================

/// Execute the `browse` subcommand and either opens a repository link in the
/// browser or prints it to stdout.
pub fn browse_repository(mut args: BrowseCommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from_disk().context("Failed to load configuration")?;
    let remote_name = args.remote.clone().unwrap_or_else(|| {
        config
            .get_string("browse/remote", None)
            .unwrap_or(DEFAULT_REMOTE.to_string())
    });
    let remote = git::get_remote_data(&remote_name)
        .with_context(|| format!("Failed to get remote URL for remote '{}'", &remote_name))?;

    args.merge_with_config(&config, Some(&remote));

    let api_type = match args.api {
        Some(api_type) => api_type,
        None => forge::guess_api_type_from_host(&remote.host)
            .with_context(|| format!("Failed to guess forge from host: {}", &remote.host))?,
    };

    if let Some(path) = args.path.as_ref() {
        return browse_path(
            &remote,
            &api_type,
            path,
            args.commit.as_deref(),
            args.no_browser,
        );
    }

    if let Some(commit_ish) = args.commit {
        return browse_commitish(&remote, &api_type, &commit_ish, args.no_browser);
    }

    if let Some(issue_number) = args.issues {
        return match issue_number {
            Some(issue_number) => browse_issue(&remote, &api_type, issue_number, args.no_browser),
            None => browse_issues(&remote, &api_type, args.no_browser),
        };
    }

    if let Some(pr_number) = args.prs {
        return match pr_number {
            Some(pr_number) => browse_pr(&remote, &api_type, pr_number, args.no_browser),
            None => browse_prs(&remote, &api_type, args.no_browser),
        };
    }

    browse_home(&remote, &api_type, args.no_browser)
}

fn browse_home(remote: &GitRemoteData, api_type: &ApiType, no_browser: bool) -> anyhow::Result<()> {
    let get_home_url = match api_type {
        ApiType::GitHub => github::get_url_for_home,
        ApiType::GitLab => gitlab::get_url_for_home,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_home,
    };
    let url = get_home_url(remote);

    print_or_open(&url, no_browser)
}

fn browse_commitish(
    remote: &GitRemoteData,
    api_type: &ApiType,
    commit_ish: &str,
    no_browser: bool,
) -> anyhow::Result<()> {
    let get_commit_url = match api_type {
        ApiType::GitHub => github::get_url_for_commit,
        ApiType::GitLab => gitlab::get_url_for_commit,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_commit,
    };
    let commit = git::rev_parse(commit_ish)
        .with_context(|| format!("Failed to resolve commit-ish: {commit_ish}"))?;
    let url = get_commit_url(remote, &commit);

    print_or_open(&url, no_browser)
}

fn browse_issue(
    remote: &GitRemoteData,
    api_type: &ApiType,
    issue_number: u32,
    no_browser: bool,
) -> anyhow::Result<()> {
    let get_issue_url = match api_type {
        ApiType::GitHub => github::get_url_for_issue,
        ApiType::GitLab => gitlab::get_url_for_issue,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_issue,
    };
    let url = get_issue_url(remote, issue_number);

    print_or_open(&url, no_browser)
}

fn browse_issues(
    remote: &GitRemoteData,
    api_type: &ApiType,
    no_browser: bool,
) -> anyhow::Result<()> {
    let get_issues_url = match api_type {
        ApiType::GitHub => github::get_url_for_issues,
        ApiType::GitLab => gitlab::get_url_for_issues,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_issues,
    };
    let url = get_issues_url(remote);

    print_or_open(&url, no_browser)
}

fn browse_pr(
    remote: &GitRemoteData,
    api_type: &ApiType,
    pr_number: u32,
    no_browser: bool,
) -> anyhow::Result<()> {
    let get_pr_url = match api_type {
        ApiType::GitHub => github::get_url_for_pr,
        ApiType::GitLab => gitlab::get_url_for_pr,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_pr,
    };
    let url = get_pr_url(remote, pr_number);

    print_or_open(&url, no_browser)
}

fn browse_prs(remote: &GitRemoteData, api_type: &ApiType, no_browser: bool) -> anyhow::Result<()> {
    let get_prs_url = match api_type {
        ApiType::GitHub => github::get_url_for_prs,
        ApiType::GitLab => gitlab::get_url_for_prs,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_prs,
    };
    let url = get_prs_url(remote);

    print_or_open(&url, no_browser)
}

fn browse_path(
    remote: &GitRemoteData,
    api_type: &ApiType,
    path: &str,
    commit_ish: Option<&str>,
    no_browser: bool,
) -> anyhow::Result<()> {
    let (file_path, line_number) = match path.rsplit_once(':') {
        Some((path_part, line_part)) => {
            if let Ok(line_nr) = line_part.parse::<u32>() {
                (path_part, Some(line_nr))
            } else {
                (path, None)
            }
        }
        None => (path, None),
    };
    let path_buf = PathBuf::from(file_path)
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize the given file path: {}", file_path))?;
    let file_path = path_buf
        .strip_prefix(git::get_absolute_repo_root()?)
        .context("Failed to resolve relative file path")?;
    let file_path = path_with_forward_slashes(file_path);
    let get_path_url = match api_type {
        ApiType::GitHub => github::get_url_for_path,
        ApiType::GitLab => gitlab::get_url_for_path,
        ApiType::Forgejo | ApiType::Gitea => gitea::get_url_for_path,
    };
    let commit = match commit_ish {
        Some(c) => {
            &git::rev_parse(c).with_context(|| format!("Failed to resolve commit-ish: {c}"))?
        }
        None => "HEAD",
    };

    let url = get_path_url(remote, &file_path, commit, line_number);

    print_or_open(&url, no_browser)
}

fn print_or_open(url: &str, no_browser: bool) -> anyhow::Result<()> {
    if no_browser {
        println!("{url}");
    } else {
        open::that(url)?;
    }

    Ok(())
}

fn path_with_forward_slashes(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
