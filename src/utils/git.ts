import { execSync } from "node:child_process";

import { CliError, exitWith } from "./debug.js";

interface ExecSyncError extends Error {
    status: number | null;
    stderr: Buffer | string;
}

function isExecSyncError(error: unknown): error is ExecSyncError {
    return error instanceof Error && "status" in error && "stderr" in error;
}

function executeGitCommand(command: string, errorContext: string): string {
    try {
        const output = execSync(command, { encoding: "utf8" }).trim();

        return output;
    } catch (e) {
        if (!isExecSyncError(e)) {
            throw e;
        }

        const exitCode = e.status ?? 1;
        const stderr = String(e.stderr);
        const message = e.message;

        exitWith(
            new GitError({
                message: `${errorContext}: ${stderr || message}`,
                command,
                exitCode,
                cause: e,
            }),
        );
    }
}

/**
 * Gets the URL for a git remote.
 *
 * @param remoteName - Remote name. Defaults to "origin".
 * @returns Remote URL (SSH or HTTPS format)
 */
export function getRemoteUrl(remoteName: string = "origin"): string {
    return executeGitCommand(
        `git remote get-url ${remoteName}`,
        `Failed to get remote URL for "${remoteName}"`,
    );
}

/**
 * Gets the current git branch name.
 *
 * @returns Current branch name
 */
export function getCurrentBranch(): string {
    return executeGitCommand(
        "git branch --show-current",
        "Failed to get current branch",
    );
}

function fetchRemoteHeadBranch(remote: string): string | undefined {
    try {
        return (
            execSync(
                // https://stackoverflow.com/questions/28666357/how-to-get-default-git-branch/50056710#50056710
                `LC_ALL=C git remote show ${remote} | sed -n '/HEAD branch/s/.*: //p'`,
                { encoding: "utf8" },
            ).trim() || undefined
        );
    } catch {
        return undefined;
    }
}

function branchExists(branchName: string): boolean {
    try {
        return !!execSync(`git rev-parse --verify ${branchName}`, {
            encoding: "utf8",
        });
    } catch {
        return false;
    }
}

/**
 * Determines the repository's default branch.
 *
 * Detection strategy (in order):
 * 1. Query remote HEAD branch
 * 2. Check if "main" exists locally
 * 3. Check if "master" exists locally
 *
 * @param remote - Remote name to query. Defaults to "origin".
 * @returns Default branch name
 */
export function getDefaultBranch(remote = "origin"): string {
    const defaultRemoteBranch = fetchRemoteHeadBranch(remote);

    if (defaultRemoteBranch) {
        return defaultRemoteBranch;
    } else if (branchExists("main")) {
        return "main";
    } else if (branchExists("master")) {
        return "master";
    }

    exitWith(
        new GitError({
            message: "Failed to determine default branch.",
            command: `git remote show ${remote}`,
            exitCode: 1,
        }),
    );
}

/**
 * Checks if the current directory is inside a git repository.
 *
 * @returns True if in a git repository, false otherwise
 */
export function isGitRepository(): boolean {
    try {
        execSync("git status", { encoding: "utf8" });

        return true;
    } catch {
        return false;
    }
}

/**
 * Pushes a branch to a remote repository.
 *
 * @param branch - Branch name to push
 * @param remote - Remote name. Defaults to "origin".
 * @param setUpstream - Whether to set upstream tracking. Defaults to true.
 */
export function pushBranch(
    branch: string,
    remote: string = "origin",
    setUpstream: boolean = true,
): void {
    executeGitCommand(
        `git push ${setUpstream ? "-u" : ""} ${remote} ${branch}`.trim(),
        `Failed to push branch "${branch}" to "${remote}"`,
    );
}

/**
 * Fetches a pull request ref into a new local branch.
 *
 * @param ref - Remote ref spec to fetch (e.g., "pull/123/head")
 * @param branchName - Name for the new local branch
 * @param remote - Remote name. Defaults to "origin".
 */
export function fetchPullRequest(
    ref: string,
    branchName: string,
    remote: string = "origin",
): void {
    executeGitCommand(
        `git fetch ${remote} ${ref}:${branchName}`,
        `Failed to fetch pull request ref ${ref}`,
    );
}

/**
 * Checks out a git branch.
 *
 * @param branchName - Branch name to checkout
 */
export function checkoutBranch(branchName: string): void {
    executeGitCommand(
        `git checkout ${branchName}`,
        `Failed to checkout branch "${branchName}"`,
    );
}

export class GitError extends CliError {
    readonly exitCode = 1;
    readonly userHint =
        "Make sure you are in a git repository with a configured remote.";
    readonly command: string;
    readonly gitExitCode: number;
    readonly cause?: unknown;

    constructor({
        message,
        command,
        exitCode,
        cause,
    }: {
        message: string;
        command: string;
        exitCode: number;
        cause?: unknown;
    }) {
        super(`Git command failed: ${command}\n${message}`);

        this.name = "GitError";
        this.command = command;
        this.gitExitCode = exitCode;
        this.cause = cause;
    }

    getDebugInfo(): Record<string, unknown> {
        const info: Record<string, unknown> = {
            command: this.command,
            exitCode: this.gitExitCode,
        };

        if (this.cause) {
            info.cause = this.cause;
        }

        return info;
    }
}
