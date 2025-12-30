import { execSync } from "node:child_process";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
    cleanupGitRepo,
    getApiUrl,
    runGitForge,
    setupGitRepoWithBareRemote,
} from "../utils.js";

describe.each([
    { forge: "github" } as const,
    { forge: "gitea" } as const,
    { forge: "gitlab" } as const,
])("PR Checkout Command ($forge)", ({ forge }) => {
    let localRepoDir: string;
    let remoteRepoDir: string;
    let prCommitSha: string;
    const prNumber = "42";
    const remotePrRef =
        forge === "gitlab" ?
            `merge-requests/${prNumber}/head`
        :   `pull/${prNumber}/head`;

    beforeEach(() => {
        [localRepoDir, remoteRepoDir] = setupGitRepoWithBareRemote({ forge });

        const branch = "feature-branch";

        // Create PR branch on remote
        execSync(
            `git checkout -b ${branch} && git commit --allow-empty -m 'Some commit' && git push origin ${branch} && git checkout main`,
            {
                cwd: localRepoDir,
                stdio: "ignore",
            },
        );

        // Update URL to fetch from file system rather than from a server
        execSync(`git remote set-url origin ${remoteRepoDir}`, {
            cwd: localRepoDir,
            stdio: "ignore",
        });

        // Create ref according to forge naming convention
        prCommitSha = execSync(`git rev-parse ${branch}`, {
            cwd: remoteRepoDir,
            encoding: "utf-8",
        }).trim();

        execSync(`git update-ref refs/${remotePrRef} ${prCommitSha}`, {
            cwd: remoteRepoDir,
        });
    });

    afterEach(() => {
        if (localRepoDir) {
            cleanupGitRepo(localRepoDir);
            localRepoDir = "";
        }

        if (remoteRepoDir) {
            cleanupGitRepo(remoteRepoDir);
            remoteRepoDir = "";
        }
    });

    it("Should display help", () => {
        const result = runGitForge({
            args: ["pr", "checkout", "--help"],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should checkout a PR", () => {
        const result = runGitForge({
            args: [
                "pr",
                "checkout",
                prNumber,
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(getCurrentBranch(localRepoDir)).toBe(`pr-${prNumber}`);
        expect(getCurrentCommit(localRepoDir)).toBe(prCommitSha);
    });

    it("Should checkout a PR using 'p' alias", () => {
        const result = runGitForge({
            args: [
                "pr",
                "co",
                prNumber,
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(getCurrentBranch(localRepoDir)).toBe(`pr-${prNumber}`);
        expect(getCurrentCommit(localRepoDir)).toBe(prCommitSha);
    });

    it("Should checkout a PR using 'pr co' alias", () => {
        const result = runGitForge({
            args: [
                "pr",
                "co",
                prNumber,
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(getCurrentBranch(localRepoDir)).toBe(`pr-${prNumber}`);
        expect(getCurrentCommit(localRepoDir)).toBe(prCommitSha);
    });

    it("Should checkout a PR with a given remote", () => {
        const result = runGitForge({
            args: [
                "pr",
                "co",
                prNumber,
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--remote",
                "origin",
            ],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(getCurrentBranch(localRepoDir)).toBe(`pr-${prNumber}`);
        expect(getCurrentCommit(localRepoDir)).toBe(prCommitSha);
    });

    it("Should fail checking out a PR when given remote doesn't exist", () => {
        const result = runGitForge({
            args: [
                "pr",
                "co",
                prNumber,
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--remote",
                "blahblah",
            ],
            cwd: localRepoDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
    });

    it("Should fail when PR ref doesn't exist", () => {
        const result = runGitForge({
            args: [
                "pr",
                "checkout",
                "999",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
    });
});

function getCurrentBranch(repoPath: string): string {
    return execSync("git rev-parse --abbrev-ref HEAD", {
        cwd: repoPath,
        encoding: "utf-8",
    }).trim();
}

function getCurrentCommit(repoPath: string): string {
    return execSync("git rev-parse HEAD", {
        cwd: repoPath,
        encoding: "utf-8",
    }).trim();
}
