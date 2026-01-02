#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ApiType {
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
}

pub fn guess_api_type_from_host(host: &str) -> anyhow::Result<ApiType> {
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
    fn test_guess_api_type_from_host() {
        let github_result = guess_api_type_from_host("https://github.com");

        assert!(github_result.is_ok());
        assert_eq!(github_result.unwrap(), ApiType::GitHub);

        let gitlab_result = guess_api_type_from_host("https://gitlab.com");

        assert!(gitlab_result.is_ok());
        assert_eq!(gitlab_result.unwrap(), ApiType::GitLab);

        let gitea_result = guess_api_type_from_host("https://gitea.com/");

        assert!(gitea_result.is_ok());
        assert_eq!(gitea_result.unwrap(), ApiType::Gitea);

        let forgejo_result = guess_api_type_from_host("https://codeberg.org/");

        assert!(forgejo_result.is_ok());
        assert_eq!(forgejo_result.unwrap(), ApiType::Forgejo);

        let unknown_forge_result = guess_api_type_from_host("https://localhost/");

        assert!(unknown_forge_result.is_err());
    }
}
