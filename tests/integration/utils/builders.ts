import type { GiteaIssueResponse, GiteaPrResponse } from "@/forge/gitea.js";
import type {
    GitHubIssueResponse,
    GitHubPullRequestResponse,
} from "@/forge/github.js";
import type {
    GitLabIssueResponse,
    GitLabMergeRequestResponse,
} from "@/forge/gitlab.js";

export function buildGitHubIssue(
    overrides: Partial<GitHubIssueResponse> = {},
): GitHubIssueResponse {
    const base: GitHubIssueResponse = {
        number: 1,
        title: "placeholder-issue",
        state: "open",
        labels: [],
        user: { login: "user" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        html_url: "https://github.com/user/repo/issues/1",
        body: "",
    };

    return { ...base, ...overrides };
}

export function buildGitLabIssue(
    overrides: Partial<GitLabIssueResponse> = {},
): GitLabIssueResponse {
    const base: GitLabIssueResponse = {
        iid: 1,
        title: "placeholder-issue",
        state: "opened",
        labels: [],
        author: { username: "user" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        web_url: "https://gitlab.example.com/user/repo/-/issues/1",
        description: "",
    };

    return { ...base, ...overrides };
}

export function buildGiteaIssue(
    overrides: Partial<GiteaIssueResponse> = {},
): GiteaIssueResponse {
    const base: GiteaIssueResponse = {
        number: 1,
        title: "placeholder-issue",
        state: "open",
        labels: [],
        user: { login: "user" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        html_url: "https://gitea.example.com/user/repo/issues/1",
        body: "",
        pull_request: undefined,
    };

    return { ...base, ...overrides };
}

export function buildGitHubPr(
    overrides: Partial<GitHubPullRequestResponse> = {},
): GitHubPullRequestResponse {
    const base: GitHubPullRequestResponse = {
        number: 1,
        title: "placeholder",
        state: "open",
        user: { login: "user" },
        labels: [],
        head: { ref: "feat" },
        base: { ref: "main" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        html_url: "https://github.com/user/repo/pull/1",
        body: "",
        draft: false,
        merged_at: null,
    };

    return { ...base, ...overrides };
}

export function buildGitLabMr(
    overrides: Partial<GitLabMergeRequestResponse> = {},
): GitLabMergeRequestResponse {
    const base: GitLabMergeRequestResponse = {
        iid: 1,
        title: "placeholder-mr",
        state: "opened",
        labels: [],
        author: { username: "user" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        web_url: "https://gitlab.example.com/user/repo/-/merge_requests/1",
        description: "",
        source_branch: "feat",
        target_branch: "main",
        draft: false,
        merge_status: "can_be_merged",
        merged_at: null,
    };

    return { ...base, ...overrides };
}

export function buildGiteaPr(
    overrides: Partial<GiteaPrResponse> = {},
): GiteaPrResponse {
    const base: GiteaPrResponse = {
        number: 1,
        title: "placeholder",
        state: "open",
        user: { login: "user" },
        labels: [],
        head: { ref: "feat", repo: { full_name: "user/repo" } },
        base: { ref: "main" },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        html_url: "https://gitea.example.com/user/repo/pulls/1",
        body: "",
        draft: false,
        mergeable: true,
        merged: false,
        merged_at: null,
    };

    return { ...base, ...overrides };
}
