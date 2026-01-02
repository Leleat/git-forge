//! Git operations and utilities.

use std::process::Command;

use anyhow::Context;

/// Gets and parses the remote URL
///
/// # Errors
///
/// Returns an error if the git command fails or the remote URL has an unknown
/// format.
pub fn get_remote_data(remote: &str) -> anyhow::Result<GitRemoteData> {
    let remote_url = get_remote_url(remote)
        .with_context(|| format!("Failed to get URL for remote '{}'", remote))?;

    match parse_remote_url(&remote_url) {
        Some(remote_data) => Ok(remote_data),
        None => anyhow::bail!(
            "Couldn't parse git remote URL. Unrecognized format. Supported: https and ssh. Found remote URL: {}",
            &remote_url
        ),
    }
}

/// Gets the URL for a git remote.
///
/// # Errors
///
/// Returns an error if git command fails; for instance if the remote doesn't
/// exist.
pub fn get_remote_url(remote: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .output()
        .with_context(|| format!("Failed to execute git command for remote '{}'", remote))?;

    if !output.status.success() {
        anyhow::bail!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Fetches a pull request ref and creates a local branch.
///
/// # Errors
///
/// Returns an error if the fetch operation fails.
pub fn fetch_pull_request(pr_ref: &str, branch_name: &str, remote: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["fetch", remote, &format!("{pr_ref}:{branch_name}")])
        .output()
        .with_context(|| format!("Failed to execute git fetch for ref '{}'", pr_ref))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to fetch pull request ref {pr_ref}: {stderr}");
    }

    Ok(())
}

/// Checks out a git branch.
///
/// # Errors
///
/// Returns an error if the checkout fails.
pub fn checkout_branch(branch_name: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["checkout", branch_name])
        .output()
        .with_context(|| {
            format!(
                "Failed to execute git checkout for branch '{}'",
                branch_name
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to checkout branch \"{branch_name}\": {stderr}");
    }

    Ok(())
}

/// Gets the name of the current git branch.
///
/// # Errors
///
/// Returns an error if the git operation fails or no branch is checked out.
pub fn get_current_branch() -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to execute git command to get current branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        anyhow::bail!("Failed to get current branch: {stderr}");
    }

    let branch = String::from_utf8_lossy(&output.stdout);
    let branch = branch.trim();

    if branch.is_empty() {
        anyhow::bail!("No branch checked out.");
    }

    Ok(branch.to_string())
}

/// Gets the default branch for a remote.
///
/// Attempts to determine the default branch by checking the remote's HEAD ref.
/// Falls back to checking for "main" or "master" branches.
///
/// # Errors
///
/// Returns an error if we can't determine the default branch.
pub fn get_default_branch(remote: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", &format!("refs/remotes/{remote}/HEAD")])
        .output()
        .with_context(|| format!("Failed to get default branch for '{}'", remote))?;

    if output.status.success() {
        let ref_name = String::from_utf8_lossy(&output.stdout);
        let ref_name = ref_name.trim();

        if let Some(branch) = ref_name.split('/').next_back() {
            return Ok(branch.to_string());
        }
    }

    for branch in ["main", "master"] {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("{remote}/{branch}")])
            .output()
            .with_context(|| {
                format!(
                    "Failed to get default branch when falling back to main or master for '{}'",
                    remote
                )
            })?;

        if output.status.success() {
            return Ok(branch.to_string());
        }
    }

    anyhow::bail!("Couldn't determine default branch")
}

/// Pushes a branch to a remote.
///
/// # Errors
///
/// Returns an error if the push operation fails.
pub fn push_branch(branch: &str, remote: &str, set_upstream: bool) -> anyhow::Result<()> {
    let mut args = vec!["push", remote, branch];

    if set_upstream {
        args.push("-u");
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .with_context(|| format!("Failed to execute git push for branch '{}'", branch))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to push branch \"{branch}\" to {remote}: {stderr}");
    }

    Ok(())
}

/// Parses commit-ish into their corresponding commit SHAs
///
/// # Errors
///
/// Returns an error if the push operation fails.
pub fn rev_parse(arg: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg(arg)
        .output()
        .context("Failed to execute git-rev-parse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        anyhow::bail!("Failed to git rev-parse {arg}: {stderr}");
    }

    let sha = String::from_utf8_lossy(&output.stdout);
    let sha = sha.trim();

    if sha.is_empty() {
        anyhow::bail!("No commit hash");
    }

    Ok(sha.to_string())
}

/// Gets the absolute path of the git repository
///
/// # Errors
///
/// Returns an error if the git command fails; e.g. if there is no working tree.
pub fn get_absolute_repo_root() -> anyhow::Result<String> {
    rev_parse("--show-toplevel")
}

/// Parsed data from a git remote URL.
#[derive(Debug, PartialEq)]
pub struct GitRemoteData {
    /// The hostname (e.g., "github.com", "gitlab.com").
    pub host: String,
    /// The repository path (e.g., "user/repo").
    pub path: String,
    /// The port number, if specified in the URL.
    pub port: Option<u16>,
}

/// Parses a git remote URL into its components.
///
/// Supports the following URL formats:
/// - HTTPS: `https://<host>[:<port>]/<user>/<repo>.git`
/// - SSH: `ssh://git@<host>[:<port>]/<user>/<repo>.git`
/// - Git SSH: `git@<host>:<user>/<repo>.git` (port not supported in this format)
pub fn parse_remote_url(url: &str) -> Option<GitRemoteData> {
    // https://<host>[:<port>]/<user>/<repo>.git
    if let Some(rest) = url.strip_prefix("https://") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();

        if parts.len() != 2 {
            return None;
        }

        let (host, port) = match parse_host_port(parts[0]) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let path = parts[1]
            .strip_suffix(".git")
            .unwrap_or(parts[1])
            .to_string();

        return Some(GitRemoteData { host, path, port });
    }

    // ssh://git@<host>[:<port>]/<user>/<repo>.git
    if let Some(rest) = url.strip_prefix("ssh://git@") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();

        if parts.len() != 2 {
            return None;
        }

        let (host, port) = match parse_host_port(parts[0]) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let path = parts[1]
            .strip_suffix(".git")
            .unwrap_or(parts[1])
            .to_string();

        return Some(GitRemoteData { host, path, port });
    }

    // git@<host>:<user>/<repo>.git
    if let Some(rest) = url.strip_prefix("git@") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();

        if parts.len() != 2 {
            return None;
        }

        let host = parts[0].to_string();
        let path = parts[1]
            .strip_suffix(".git")
            .unwrap_or(parts[1])
            .to_string();

        return Some(GitRemoteData {
            host,
            path,
            port: None,
        });
    }

    None
}

fn parse_host_port(host_str: &str) -> anyhow::Result<(String, Option<u16>)> {
    if let Some(colon_pos) = host_str.rfind(':') {
        let host = host_str[..colon_pos].to_string();
        let port_str = &host_str[colon_pos + 1..];

        match port_str.parse::<u16>() {
            Ok(port) => Ok((host, Some(port))),
            Err(_) => anyhow::bail!("Invalid port number: {}", port_str),
        }
    } else {
        Ok((host_str.to_string(), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_url_https() {
        // https://github.com/user/repo.git
        let result = parse_remote_url("https://github.com/user/repo.git");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );

        // https://github.com/user/repo (without .git)
        let result = parse_remote_url("https://github.com/user/repo");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );
    }

    #[test]
    fn test_parse_remote_url_https_with_port() {
        // https://gitlab.example.com:8443/user/repo.git
        let result = parse_remote_url("https://gitlab.example.com:8443/user/repo.git");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "gitlab.example.com".to_string(),
                path: "user/repo".to_string(),
                port: Some(8443),
            }
        );

        // https://localhost:3000/user/repo
        let result = parse_remote_url("https://localhost:3000/user/repo");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "localhost".to_string(),
                path: "user/repo".to_string(),
                port: Some(3000),
            }
        );
    }

    #[test]
    fn test_parse_remote_url_ssh() {
        // ssh://git@github.com/user/repo.git
        let result = parse_remote_url("ssh://git@github.com/user/repo.git");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );

        // ssh://git@github.com/user/repo (without .git)
        let result = parse_remote_url("ssh://git@github.com/user/repo");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );
    }

    #[test]
    fn test_parse_remote_url_ssh_with_port() {
        // ssh://git@gitlab.example.com:2222/user/repo.git
        let result = parse_remote_url("ssh://git@gitlab.example.com:2222/user/repo.git");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "gitlab.example.com".to_string(),
                path: "user/repo".to_string(),
                port: Some(2222),
            }
        );

        // ssh://git@localhost:22022/user/repo
        let result = parse_remote_url("ssh://git@localhost:22022/user/repo");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "localhost".to_string(),
                path: "user/repo".to_string(),
                port: Some(22022),
            }
        );
    }

    #[test]
    fn test_parse_remote_url_git_ssh() {
        // git@github.com:user/repo.git
        let result = parse_remote_url("git@github.com:user/repo.git");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );

        // git@github.com:user/repo (without .git)
        let result = parse_remote_url("git@github.com:user/repo");

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            GitRemoteData {
                host: "github.com".to_string(),
                path: "user/repo".to_string(),
                port: None,
            }
        );
    }
}
