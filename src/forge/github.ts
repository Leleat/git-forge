import {
    DEFAULT_PAGE,
    DEFAULT_PER_PAGE,
    type CreatePrParams,
    type Forge,
    type Issue,
    type IssueFilters,
    type Pr,
    type PrFilters,
    type WebUrlType,
} from "@/forge/forge.js";
import { exitWith } from "@/utils/debug.js";
import { AuthenticationError, ForgeHttpClient } from "@/utils/httpClient.js";

/**
 * GitHub API response for issues.
 *
 * @see https://docs.github.com/en/rest/issues/issues
 */
export interface GitHubIssueResponse {
    number: number;
    title: string;
    state: string;
    labels: Array<{ name: string }>;
    user: { login: string };
    created_at: string;
    updated_at: string;
    html_url: string;
    body?: string;
    pull_request?: unknown;
}

/**
 * GitHub API response for pull requests.
 *
 * @see https://docs.github.com/en/rest/pulls/pulls
 */
export interface GitHubPullRequestResponse {
    number: number;
    title: string;
    state: string;
    labels: Array<{ name: string }>;
    user: { login: string };
    created_at: string;
    updated_at: string;
    html_url: string;
    body: string;
    head: { ref: string };
    base: { ref: string };
    draft?: boolean;
    merged_at: string | null;
}

export class GitHubForge implements Forge {
    readonly name = "GitHub";

    private readonly _apiBase: string;
    private readonly _webBase: string;
    private readonly _repoPath: string;
    private readonly _httpClient: ForgeHttpClient;

    constructor({
        host,
        path,
        useAuth = false,
    }: {
        host: string;
        path: string;
        useAuth?: boolean;
    }) {
        this._repoPath = path;

        if (host === "github.com") {
            this._apiBase = "https://api.github.com";
            this._webBase = "https://github.com";
        } else {
            this._apiBase = `https://${host}/api/v3`;
            this._webBase = `https://${host}`;
        }

        const headers: Record<string, string> = {
            Accept: "application/vnd.github.v3+json",
        };

        if (useAuth && process.env.GITHUB_TOKEN) {
            headers["Authorization"] = `Bearer ${process.env.GITHUB_TOKEN}`;
        } else if (useAuth && !process.env.GITHUB_TOKEN) {
            exitWith(
                new AuthenticationError({
                    forgeName: "GitHub",
                    message: "Authentication enabled but GITHUB_TOKEN not set",
                }),
            );
        }

        this._httpClient = new ForgeHttpClient({
            forgeName: "GitHub",
            headers,
        });
    }

    /**
     * Lists issues from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Issue objects
     * @see https://docs.github.com/en/rest/issues/issues#list-repository-issues
     */
    async listIssues(filters?: IssueFilters): Promise<Issue[]> {
        const params = new URLSearchParams({
            state: filters?.state ?? "open",
            per_page: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
        });

        if (filters?.author) {
            params.set("creator", filters.author);
        }

        if (filters?.labels && filters.labels.length > 0) {
            params.set("labels", filters.labels.join(","));
        }

        const url = `${this._apiBase}/repos/${this._repoPath}/issues?${params}`;
        const issuesAndPrs =
            await this._httpClient.getJson<GitHubIssueResponse[]>(url);

        return issuesAndPrs
            .filter((item) => !item.pull_request)
            .map(mapToIssue);
    }

    /**
     * Lists pull requests from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Pr objects
     * @see https://docs.github.com/en/rest/pulls/pulls#list-pull-requests
     */
    async listPrs(filters?: PrFilters): Promise<Pr[]> {
        let apiState: string = filters?.state ?? "open";
        let filterMerged = false;

        if (filters?.state === "merged") {
            apiState = "closed";
            filterMerged = true;
        } else if (filters?.state === "all") {
            apiState = "all";
        }

        const params = new URLSearchParams({
            state: apiState,
            per_page: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
        });
        const url = `${this._apiBase}/repos/${this._repoPath}/pulls?${params}`;
        const pullRequests =
            await this._httpClient.getJson<GitHubPullRequestResponse[]>(url);
        let filtered = pullRequests;

        if (filterMerged) {
            filtered = filtered.filter((pr) => pr.merged_at !== null);
        } else if (filters?.state === "closed") {
            filtered = filtered.filter((pr) => pr.merged_at === null);
        }

        if (filters?.author) {
            filtered = filtered.filter(
                (pr) => pr.user.login === filters.author,
            );
        }

        if (filters?.labels && filters.labels.length > 0) {
            filtered = filtered.filter((pr) =>
                filters.labels!.every((label) =>
                    pr.labels.some((l) => l.name === label),
                ),
            );
        }

        if (filters?.draft !== undefined) {
            filtered = filtered.filter((pr) => pr.draft === filters.draft);
        }

        return filtered.map(mapToPullRequest);
    }

    /**
     * Creates a new pull request.
     *
     * @param params - Pull request creation parameters
     * @returns Promise resolving to created Pr object
     * @see https://docs.github.com/en/rest/pulls/pulls#create-a-pull-request
     */
    async createPr(params: CreatePrParams): Promise<Pr> {
        const url = `${this._apiBase}/repos/${this._repoPath}/pulls`;
        const body = {
            title: params.title,
            head: params.sourceBranch,
            base: params.targetBranch,
            body: params.body ?? "",
            draft: params.draft ?? false,
        };
        const prData =
            await this._httpClient.postJson<GitHubPullRequestResponse>(
                url,
                body,
            );

        return mapToPullRequest(prData);
    }

    /**
     * Returns the web URL for the repository or a specific page.
     *
     * @param type - URL type: "repository" (default), "issues", "prs", or "mrs"
     * @returns Full HTTPS URL to repository or specified page
     */
    getWebUrl(type?: WebUrlType): string {
        const baseUrl = `${this._webBase}/${this._repoPath}`;

        switch (type) {
            case "issues":
                return `${baseUrl}/issues`;
            case "prs":
            case "mrs":
                return `${baseUrl}/pulls`;
            default:
                return baseUrl;
        }
    }

    /**
     * Returns the git reference for fetching a pull request.
     *
     * @param prNumber - Pull request number
     * @returns Git ref spec (e.g., "pull/123/head")
     */
    getPrRef(prNumber: string): string {
        return `pull/${prNumber}/head`;
    }
}

function mapToIssue(issue: GitHubIssueResponse): Issue {
    return {
        id: issue.number.toString(),
        title: issue.title,
        state: issue.state,
        author: issue.user.login,
        url: issue.html_url,
        labels: issue.labels.map((l) => l.name),
        createdAt: new Date(issue.created_at),
        updatedAt: new Date(issue.updated_at),
    };
}

function mapToPullRequest(pr: GitHubPullRequestResponse): Pr {
    return {
        id: pr.number.toString(),
        title: pr.title,
        state: pr.merged_at ? "merged" : pr.state,
        author: pr.user.login,
        url: pr.html_url,
        labels: pr.labels.map((l) => l.name),
        createdAt: new Date(pr.created_at),
        updatedAt: new Date(pr.updated_at),
        sourceBranch: pr.head.ref,
        targetBranch: pr.base.ref,
        draft: !!pr.draft,
        mergeable: !!pr.merged_at,
    };
}
