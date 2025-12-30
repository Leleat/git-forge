use anyhow::Context;

use crate::{
    cli::{
        forge::{gitea::GiteaClient, github::GitHubClient, gitlab::GitLabClient},
        issue::{Issue, IssueState},
        pr::{Pr, PrState},
        web::WebTarget,
    },
    git,
};

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ApiType {
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
}

/// Creates a forge client.
///
/// This factory function automatically detects the API type from the remote
/// URL's hostname, or uses an explicitly specified API type and URL.
pub fn create_forge_client(
    remote_name: String,
    api: Option<ApiType>,
    api_url: Option<String>,
) -> anyhow::Result<Box<dyn ForgeClient>> {
    let remote_url = git::get_remote_url(&remote_name)
        .with_context(|| format!("Failed to get URL for remote '{}'", remote_name))?;
    let remote_data = git::parse_remote_url(&remote_url);
    let api = match api {
        Some(v) => v,
        None => match remote_data.as_ref() {
            Some(remote_data) => {
                guess_forge_type_from_host(&remote_data.host).with_context(|| {
                    format!(
                        "Failed to determine the forge type from the host '{}'",
                        &remote_data.host
                    )
                })?
            }
            None => anyhow::bail!(
                "Couldn't determine the forge type and none was explicitely provided."
            ),
        },
    };
    let forge_client: Box<dyn ForgeClient> = match api {
        ApiType::Forgejo | ApiType::Gitea => Box::new(GiteaClient::new(remote_data, api_url)),
        ApiType::GitHub => Box::new(GitHubClient::new(remote_data, api_url)),
        ApiType::GitLab => Box::new(GitLabClient::new(remote_data, api_url)),
    };

    Ok(forge_client)
}

pub trait ForgeClient {
    /// Fetches issues from the forge.
    fn get_issues(
        &self,
        use_auth: bool,
        author: Option<&str>,
        labels: &[String],
        page: u32,
        per_page: u32,
        state: IssueState,
    ) -> anyhow::Result<Vec<Issue>>;

    /// Fetches pull requests from the forge.
    #[allow(clippy::too_many_arguments)]
    fn get_prs(
        &self,
        use_auth: bool,
        author: Option<&str>,
        labels: &[String],
        page: u32,
        per_page: u32,
        state: PrState,
        draft: bool,
    ) -> anyhow::Result<Vec<Pr>>;

    /// Creates a new pull request on the forge.
    fn create_pr(
        &self,
        title: &str,
        source_branch: &str,
        target_branch: &str,
        body: Option<&str>,
        draft: bool,
    ) -> anyhow::Result<Pr>;

    /// Returns the git ref string for fetching a pull request.
    fn get_pr_ref(&self, pr_number: u32) -> String;

    /// Generates a web URL for viewing the specified target.
    fn get_web_url(&self, target: WebTarget) -> anyhow::Result<String>;
}

fn guess_forge_type_from_host(host: &str) -> anyhow::Result<ApiType> {
    let host = host.to_lowercase();

    if host.contains("github") {
        return Ok(ApiType::GitHub);
    } else if host.contains("gitlab") {
        return Ok(ApiType::GitLab);
    } else if host.contains("gitea") {
        return Ok(ApiType::Gitea);
    } else if host.contains("forgejo") || host.contains("codeberg") {
        return Ok(ApiType::Forgejo);
    }

    anyhow::bail!(
        "Unable to detect forge type from hostname. Supported: github, gitlab, gitea, forgejo. Use --api flag to specify explicitly"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_forge_type_from_host() {
        let github_result = guess_forge_type_from_host("https://github.com");

        assert!(github_result.is_ok());
        assert_eq!(github_result.unwrap(), ApiType::GitHub);

        let gitlab_result = guess_forge_type_from_host("https://gitlab.com");

        assert!(gitlab_result.is_ok());
        assert_eq!(gitlab_result.unwrap(), ApiType::GitLab);

        let gitea_result = guess_forge_type_from_host("https://gitea.com/");

        assert!(gitea_result.is_ok());
        assert_eq!(gitea_result.unwrap(), ApiType::Gitea);

        let forgejo_result = guess_forge_type_from_host("https://codeberg.org/");

        assert!(forgejo_result.is_ok());
        assert_eq!(forgejo_result.unwrap(), ApiType::Forgejo);

        let unknown_forge_result = guess_forge_type_from_host("https://localhost/");

        assert!(unknown_forge_result.is_err());
    }
}
