# Tests for git-forge

This repository contains the e2e tests for [git-forge](https://github.com/Leleat/git-forge).

The tests start a local express server on `http://localhost:{3001,3002,3003}` simulating the API responses of the supported git forges. Each test creates a temporary git repository and runs `git-forge` in it. Each forge returns the same (relative) response, so that the output of `git-forge` is the same for each forge.

## Usage

```sh
# First, install the dependencies
npm install

# Then, either run the tests in watch mode
npm run test

# Or alternatively, run the tests once
npm run test:once
```

## API Documentations

- GitHub
    - [List Issues](https://docs.github.com/en/rest/issues/issues?apiVersion=2022-11-28#list-repository-issues)
    - [List PRs](https://docs.github.com/en/rest/pulls/pulls?apiVersion=2022-11-28#list-pull-requests)
    - [Create PRs](https://docs.github.com/en/rest/pulls/pulls?apiVersion=2022-11-28#create-a-pull-request)
- GitLab
    - [List Issues](https://docs.gitlab.com/api/issues/#list-project-issues)
    - [List MRs](https://docs.gitlab.com/api/merge_requests/#list-project-merge-requests)
    - [Create MR](https://docs.gitlab.com/api/merge_requests/#create-mr)
- Gitea
    - [Issues](https://docs.gitea.com/api/#tag/issue/operation/issueSearchIssues)
    - [PRs](https://docs.gitea.com/api/#tag/repository/operation/repoNewPinAllowed)
