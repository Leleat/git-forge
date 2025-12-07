import { exitWith } from "@/utils/debug.js";
import { AuthenticationError, ForgeHttpClient } from "@/utils/httpClient.js";
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
} from "./forge.js";

/**
 * Gitea/Forgejo API response for issues.
 *
 * @see https://docs.gitea.com/api/
 */
export interface GiteaIssueResponse {
    number: number;
    title: string;
    state: string;
    labels: Array<{ name: string }>;
    user: { login: string };
    created_at: string;
    updated_at: string;
    html_url: string;
    body: string;
    pull_request?: unknown;
}

/**
 * Gitea/Forgejo API response for pull requests.
 *
 * @see https://docs.gitea.com/api/
 */
export interface GiteaPrResponse {
    number: number;
    title: string;
    state: string;
    labels: Array<{ name: string }>;
    user: { login: string };
    created_at: string;
    updated_at: string;
    html_url: string;
    body: string;
    head: { ref: string; repo: { full_name: string } };
    base: { ref: string };
    draft: boolean;
    mergeable: boolean;
    merged: boolean;
    merged_at: string | null;
}

export class GiteaForge implements Forge {
    readonly name: string;

    private readonly _apiBase: string;
    private readonly _webBase: string;
    private readonly _repoPath: string;
    private readonly _httpClient: ForgeHttpClient;

    constructor({
        host,
        path,
        forgeName,
        useAuth = false,
    }: {
        host: string;
        path: string;
        forgeName: "Gitea" | "Forgejo";
        useAuth?: boolean;
    }) {
        this.name = forgeName;
        this._repoPath = path;
        this._apiBase = `https://${host}/api/v1`;
        this._webBase = `https://${host}`;

        const headers: Record<string, string> = {};

        if (useAuth && process.env.GITEA_TOKEN) {
            headers["Authorization"] = `Bearer ${process.env.GITEA_TOKEN}`;
        } else if (useAuth && !process.env.GITEA_TOKEN) {
            exitWith(
                new AuthenticationError({
                    forgeName,
                    message: "Authentication enabled but GITEA_TOKEN not set",
                }),
            );
        }

        this._httpClient = new ForgeHttpClient({
            forgeName: this.name,
            headers,
        });
    }

    /**
     * Lists issues from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Issue objects
     * @see https://docs.gitea.com/api/#tag/issue/operation/issueListIssues
     */
    async listIssues(filters?: IssueFilters): Promise<Issue[]> {
        const params = new URLSearchParams({
            state: filters?.state ?? "open",
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
            limit: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
            type: "issues",
        });

        if (filters?.author) {
            params.set("created_by", filters.author);
        }

        if (filters?.labels && filters.labels.length > 0) {
            params.set("labels", filters.labels.join(","));
        }

        const url = `${this._apiBase}/repos/${this._repoPath}/issues?${params}`;
        const issues =
            await this._httpClient.getJson<GiteaIssueResponse[]>(url);

        return issues.map(mapToIssue);
    }

    /**
     * Lists pull requests from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Pr objects
     * @see https://docs.gitea.com/api/#tag/repository/operation/repoListPullRequests
     */
    async listPrs(filters?: PrFilters): Promise<Pr[]> {
        let apiState: string;

        if (filters?.state === "merged") {
            apiState = "closed";
        } else if (filters?.state === "closed") {
            apiState = "closed";
        } else if (filters?.state === "all") {
            apiState = "all";
        } else {
            apiState = "open";
        }

        const params = new URLSearchParams({
            state: apiState,
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
            limit: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
        });
        const url = `${this._apiBase}/repos/${this._repoPath}/pulls?${params}`;
        const pullRequests =
            await this._httpClient.getJson<GiteaPrResponse[]>(url);

        let filtered = pullRequests;

        if (filters?.state === "merged") {
            filtered = filtered.filter((pr) => pr.merged === true);
        } else if (filters?.state === "closed") {
            filtered = filtered.filter((pr) => pr.merged === false);
        }

        if (filters?.author) {
            filtered = filtered.filter(
                (pr) => pr.user.login === filters.author,
            );
        }

        if (filters?.labels && filters.labels.length > 0) {
            filtered = filtered.filter((pr) =>
                filters.labels!.some((label) =>
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
     * @see https://docs.gitea.com/api/#tag/repository/operation/repoCreatePullRequest
     */
    async createPr(params: CreatePrParams): Promise<Pr> {
        const url = `${this._apiBase}/repos/${this._repoPath}/pulls`;
        const body = {
            title: params.draft ? `WIP: ${params.title}` : params.title,
            head: params.sourceBranch,
            base: params.targetBranch,
            body: params.body ?? "",
        };
        const prData = await this._httpClient.postJson<GiteaPrResponse>(
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

function mapToIssue(issue: GiteaIssueResponse): Issue {
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

function mapToPullRequest(pr: GiteaPrResponse): Pr {
    return {
        id: pr.number.toString(),
        title: pr.title,
        state: pr.merged ? "merged" : pr.state,
        author: pr.user.login,
        url: pr.html_url,
        labels: pr.labels.map((l) => l.name),
        createdAt: new Date(pr.created_at),
        updatedAt: new Date(pr.updated_at),
        sourceBranch: pr.head.ref,
        targetBranch: pr.base.ref,
        draft: pr.draft,
        mergeable: pr.mergeable,
    };
}
