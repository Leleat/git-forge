use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
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
        "Unable to detect forge type from hostname '{host}'. Supported keywords: github, gitlab, gitea, forgejo. Use --api to specify the forge type explicitly."
    )
}

/// Gets a forge-specific function
///
/// # Example
///
/// ```rust,ignore
/// let api_type = ApiType::GitHub;
/// let get_issues = forge::function!(api_type, get_issues);
///
/// assert_eq!(get_issues, crate::cli::forge::github::get_issues);
/// ```
macro_rules! function {
    ($type:expr, $fn_name:ident) => {
        match $type {
            ApiType::GitHub => github::$fn_name,
            ApiType::GitLab => gitlab::$fn_name,
            ApiType::Gitea | ApiType::Forgejo => gitea::$fn_name,
        }
    };
}

pub(crate) use function;

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
