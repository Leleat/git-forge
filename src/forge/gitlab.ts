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
 * GitLab API response for issues.
 *
 * @see https://docs.gitlab.com/api/issues/
 */
export interface GitLabIssueResponse {
    iid: number;
    title: string;
    state: string;
    labels: string[];
    author: { username: string };
    created_at: string;
    updated_at: string;
    web_url: string;
    description: string;
}

/**
 * GitLab API response for pull requests.
 *
 * @see https://docs.gitlab.com/api/merge_requests/
 */
export interface GitLabMergeRequestResponse {
    iid: number;
    title: string;
    state: string;
    labels: string[];
    author: { username: string };
    created_at: string;
    updated_at: string;
    web_url: string;
    description: string;
    source_branch: string;
    target_branch: string;
    draft: boolean;
    merge_status: string;
    merged_at: string | null;
}

type GitLabState = "opened" | "closed" | "all";

export class GitLabForge implements Forge {
    readonly name = "GitLab";

    private readonly _apiBase: string;
    private readonly _webBase: string;
    private readonly _encodedPath: string;
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
        this._apiBase = `https://${host}/api/v4`;
        this._webBase = `https://${host}`;
        this._encodedPath = encodeURIComponent(path);

        const headers: Record<string, string> = {};

        if (useAuth && process.env.GITLAB_TOKEN) {
            headers["Authorization"] = `Bearer ${process.env.GITLAB_TOKEN}`;
        } else if (useAuth && !process.env.GITLAB_TOKEN) {
            exitWith(
                new AuthenticationError({
                    forgeName: "GitLab",
                    message: "Authentication enabled but GITLAB_TOKEN not set",
                }),
            );
        }

        this._httpClient = new ForgeHttpClient({
            forgeName: "GitLab",
            headers,
        });
    }

    /**
     * Lists issues from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Issue objects
     * @see https://docs.gitlab.com/api/issues/#list-project-issues
     */
    async listIssues(filters?: IssueFilters): Promise<Issue[]> {
        let state: GitLabState;

        if (filters?.state === "closed") {
            state = "closed";
        } else if (filters?.state === "all") {
            state = "all";
        } else {
            state = "opened";
        }

        const params = new URLSearchParams({
            state,
            per_page: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
        });

        if (filters?.author) {
            params.set("author_username", filters.author);
        }

        if (filters?.labels && filters.labels.length > 0) {
            params.set("labels", filters.labels.join(","));
        }

        const url = `${this._apiBase}/projects/${this._encodedPath}/issues?${params}`;
        const issues =
            await this._httpClient.getJson<GitLabIssueResponse[]>(url);

        return issues.map(mapToIssue);
    }

    /**
     * Lists pull requests from the repository.
     *
     * @param filters - Optional filter criteria
     * @returns Promise resolving to array of normalized Pr objects
     * @see https://docs.gitlab.com/api/merge_requests/#list-merge-requests
     */
    async listPrs(filters?: PrFilters): Promise<Pr[]> {
        let apiState: string;

        if (filters?.state === "closed") {
            apiState = "closed";
        } else if (filters?.state === "merged") {
            apiState = "merged";
        } else if (filters?.state === "all") {
            apiState = "all";
        } else if (filters?.state === "locked") {
            apiState = "locked";
        } else {
            apiState = "opened";
        }

        const params = new URLSearchParams({
            state: apiState,
            per_page: (filters?.perPage ?? DEFAULT_PER_PAGE).toString(),
            page: (filters?.page ?? DEFAULT_PAGE).toString(),
        });

        if (filters?.author) {
            params.set("author_username", filters.author);
        }

        if (filters?.labels && filters.labels.length > 0) {
            params.set("labels", filters.labels.join(","));
        }

        if (filters?.draft !== undefined) {
            params.set("wip", filters.draft ? "yes" : "no");
        }

        const url = `${this._apiBase}/projects/${this._encodedPath}/merge_requests?${params}`;
        const mergeRequests =
            await this._httpClient.getJson<GitLabMergeRequestResponse[]>(url);

        return mergeRequests.map(mapToMergeRequest);
    }

    /**
     * Creates a new merge request.
     *
     * @param params - Pull request creation parameters
     * @returns Promise resolving to created Pr object
     * @see https://docs.gitlab.com/api/merge_requests/#create-mr
     */
    async createPr(params: CreatePrParams): Promise<Pr> {
        const url = `${this._apiBase}/projects/${this._encodedPath}/merge_requests`;
        const body = {
            source_branch: params.sourceBranch,
            target_branch: params.targetBranch,
            title: params.draft ? `Draft: ${params.title}` : params.title,
            description: params.body ?? "",
        };
        const mrData =
            await this._httpClient.postJson<GitLabMergeRequestResponse>(
                url,
                body,
            );

        return mapToMergeRequest(mrData);
    }

    /**
     * Returns the web URL for the repository or a specific page.
     *
     * @param type - URL type: "repository" (default), "issues", "prs", or "mrs"
     * @returns Full HTTPS URL to repository or specified page
     */
    getWebUrl(type?: WebUrlType): string {
        const baseUrl = `${this._webBase}/${decodeURIComponent(this._encodedPath)}`;

        switch (type) {
            case "issues":
                return `${baseUrl}/-/issues`;
            case "prs":
            case "mrs":
                return `${baseUrl}/-/merge_requests`;
            default:
                return baseUrl;
        }
    }

    /**
     * Returns the git reference for fetching a pull request.
     *
     * @param prNumber - Pull request number
     * @returns Git ref spec (e.g., "merge-requests/123/head")
     */
    getPrRef(prNumber: string): string {
        return `merge-requests/${prNumber}/head`;
    }
}

function mapToIssue(issue: GitLabIssueResponse): Issue {
    return {
        id: issue.iid.toString(),
        title: issue.title,
        state: issue.state === "opened" ? "open" : issue.state,
        author: issue.author.username,
        url: issue.web_url,
        labels: issue.labels,
        createdAt: new Date(issue.created_at),
        updatedAt: new Date(issue.updated_at),
    };
}

function mapToMergeRequest(mr: GitLabMergeRequestResponse): Pr {
    return {
        id: mr.iid.toString(),
        title: mr.title,
        state: mr.state === "opened" ? "open" : mr.state,
        author: mr.author.username,
        url: mr.web_url,
        labels: mr.labels,
        createdAt: new Date(mr.created_at),
        updatedAt: new Date(mr.updated_at),
        sourceBranch: mr.source_branch,
        targetBranch: mr.target_branch,
        draft: mr.draft,
        mergeable: mr.merge_status === "can_be_merged",
    };
}
