import { ParseArgsOptionsConfig } from "node:util";

import { CliError, exitWith } from "@/utils/debug.js";
import { getRemoteUrl } from "@/utils/git.js";

/** Argument configuration for common forge-related CLI flags. */
export const ForgeArgsOptionsConfig = {
    remote: {
        type: "string",
        default: "origin",
    },
    "forge-type": {
        type: "string",
    },
    auth: {
        type: "boolean",
        default: false,
    },
} as const satisfies ParseArgsOptionsConfig;

export type WebUrlType = "repository" | "issues" | "prs" | "mrs";

/**
 * Core abstraction for git forge platforms.
 *
 * Provides a unified interface for interacting with different forge APIs,
 * normalizing differences in data models and API endpoints.
 */
export interface Forge {
    readonly name: string;

    /**
     * Lists issues from the repository with optional filtering.
     *
     * @param filters - Optional filter criteria. Defaults to open issues.
     * @returns Promise resolving to array of normalized Issue objects.
     */
    listIssues(filters?: IssueFilters): Promise<Issue[]>;

    /**
     * Lists pull requests from the repository with optional filtering.
     *
     * @param filters - Optional filter criteria. defaults to open PRs.
     * @returns Promise resolving to array of normalized Pr objects.
     */
    listPrs(filters?: PrFilters): Promise<Pr[]>;

    /**
     * Creates a new pull request.
     *
     * @param params - Pull request creation parameters
     * @returns Promise resolving to created Pr object
     */
    createPr(params: CreatePrParams): Promise<Pr>;

    /**
     * Returns the web URL for viewing the repository in a browser.
     *
     * @param type - URL type: "repository" (default), "issues", "prs" or "mrs"
     * @returns Full HTTPS URL to repository or specified page
     */
    getWebUrl(type?: WebUrlType): string;

    /**
     * Returns the git reference for fetching a pull request.
     *
     * @param prNumber - Pull request number as string
     * @returns Git ref spec for fetching the PR branch
     */
    getPrRef(prNumber: string): string;
}

export interface Issue {
    id: string;
    title: string;
    state: string;
    author: string;
    url: string;
    labels: string[];
    createdAt: Date;
    updatedAt: Date;
}

export interface IssueFilters {
    state?: "open" | "closed" | "all";
    labels?: string[];
    author?: string;
    page?: number; // Page number for pagination (1-indexed)
    perPage?: number; // Results per page. Max may vary by forge (eg 100)
}

export interface Pr {
    id: string;
    title: string;
    state: string;
    author: string;
    url: string;
    labels: string[];
    createdAt: Date;
    updatedAt: Date;
    sourceBranch: string;
    targetBranch: string;
    draft: boolean;
    mergeable: boolean;
}

export interface PrFilters {
    state?: string;
    author?: string;
    labels?: string[];
    draft?: boolean;
    page?: number; // Page number for pagination (1-indexed)
    perPage?: number; // Results per page. Max may vary by forge (eg 100)
}

export interface CreatePrParams {
    title: string;
    sourceBranch: string;
    targetBranch: string;
    body?: string;
    draft?: boolean;
}

export const VALID_FORGES = ["github", "gitlab", "gitea", "forgejo"] as const;

export type ForgeType = (typeof VALID_FORGES)[number];

export const DEFAULT_PAGE = 1;

export const DEFAULT_PER_PAGE = 100;

export const MAX_PER_PAGE = 100;

function parseRemoteUrl(remoteUrl: string): {
    host: string;
    path: string;
} {
    // ssh url
    const sshMatch = remoteUrl.match(
        // https://docs.github.com/en/get-started/git-basics/about-remote-repositories#about-remote-repositories
        /^(ssh:\/\/)?git@([^:]+):(.+?)(?:\.git)?$/,
    );

    if (sshMatch) {
        const host = sshMatch[2];
        const path = sshMatch[3];

        return { host, path };
    }

    // https url
    try {
        const url = new URL(remoteUrl);
        const host = url.hostname;
        const path = url.pathname.replace(/^\//, "").replace(/\.git$/, "");

        return { host, path };
    } catch (error) {
        exitWith(
            new ForgeDetectionError({
                message: "Could not parse remote URL",
                remoteUrl,
                cause: error,
            }),
        );
    }
}

function detectForgeType(forgeHost: string): ForgeType {
    const host = forgeHost.toLowerCase();

    if (host.includes("github")) {
        return "github";
    }

    if (host.includes("gitlab")) {
        return "gitlab";
    }

    if (
        host.includes("gitea") ||
        host.includes("forgejo") ||
        host.includes("codeberg")
    ) {
        return "gitea";
    }

    exitWith(
        new ForgeDetectionError({
            message:
                "Unable to detect forge type from hostname. Supported: github, gitlab, gitea, forgejo. Use --forge-type flag to specify explicitly",
            remoteUrl: forgeHost,
        }),
    );
}

/**
 * Factory function for creating forge instances.
 *
 * @param remoteName - Git remote name. Defaults to "origin".
 * @param useAuth - Whether to use authentication. Defaults to false.
 * @param forgeType - Optional explicit forge type. Auto-detected if not given.
 * @returns Promise resolving to Forge implementation
 */
export async function createForge(
    remoteName: string = "origin",
    useAuth: boolean = false,
    forgeType?: ForgeType,
): Promise<Forge> {
    const remoteUrl = getRemoteUrl(remoteName);
    const { host, path } = parseRemoteUrl(remoteUrl);
    const fType = forgeType ?? detectForgeType(host);

    switch (fType) {
        case "github": {
            const { GitHubForge } = await import("./github.js");
            return new GitHubForge({ host, path, useAuth });
        }
        case "gitlab": {
            const { GitLabForge } = await import("./gitlab.js");
            return new GitLabForge({ host, path, useAuth });
        }
        case "gitea":
        case "forgejo": {
            const { GiteaForge } = await import("./gitea.js");
            return new GiteaForge({
                host,
                path,
                forgeName: fType === "gitea" ? "Gitea" : "Forgejo",
                useAuth,
            });
        }
        default:
            exitWith(
                new ForgeDetectionError({
                    message: `Unknown forge type: ${fType}`,
                    remoteUrl: host,
                }),
            );
    }
}

export class ForgeDetectionError extends CliError {
    readonly exitCode = 1;
    readonly userHint = `Use --forge-type to explicitly specify the forge type:\n  github, gitlab, gitea, forgejo`;
    readonly remoteUrl: string;
    readonly cause?: unknown;

    constructor({
        message,
        remoteUrl,
        cause,
    }: {
        message: string;
        remoteUrl: string;
        cause?: unknown;
    }) {
        super(`${message}: ${remoteUrl}`);

        this.name = "ForgeDetectionError";
        this.remoteUrl = remoteUrl;
        this.cause = cause;
    }

    getDebugInfo(): Record<string, unknown> {
        const info: Record<string, unknown> = {
            remoteUrl: this.remoteUrl,
        };

        if (this.cause) {
            info.cause = this.cause;
        }

        return info;
    }
}
