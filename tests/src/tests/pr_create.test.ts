import { execSync } from "node:child_process";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
    cleanupGitRepo,
    getApiUrl,
    runGitForge,
    setupGitRepoWithBareRemote,
} from "../utils.js";

describe.each([
    { forge: "github", token: "GIT_FORGE_GITHUB_TOKEN" } as const,
    { forge: "gitea", token: "GIT_FORGE_GITEA_TOKEN" } as const,
    { forge: "gitlab", token: "GIT_FORGE_GITLAB_TOKEN" } as const,
])("PR Create Command ($forge)", ({ forge, token }) => {
    let localRepoDir: string;
    let remoteRepoDir: string;

    beforeEach(() => {
        [localRepoDir, remoteRepoDir] = setupGitRepoWithBareRemote({ forge });
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
            args: ["pr", "create", "--help"],
            cwd: localRepoDir,
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toBeTruthy();
    });

    it("Should create PR with default settings", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Some Title",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should create PR with custom title", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Custom PR Title",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should create PR with custom body", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Some Title",
                "--body",
                "This is a test body",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should create draft PR", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Some Title",
                "--draft",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should create PR with custom target branch", () => {
        execSync("git checkout -b develop", {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        execSync("git push origin develop", {
            cwd: localRepoDir,
            stdio: "ignore",
        });
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Some Title",
                "--target",
                "develop",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should create PR with title, body, and draft combined on GitHub", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--title",
                "Combined Options Test",
                "--body",
                "This tests multiple flags together",
                "--draft",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should fail if no token is set", () => {
        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api-url",
                getApiUrl(forge),
            ],
            cwd: localRepoDir,
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
    });

    it("Should fail when creating PR with the same source and target branch", () => {
        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--target",
                "main",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
    });

    it("Should create PR with --fill", () => {
        execSync(
            "git checkout -b feature/add-logging && git commit --allow-empty -m 'Add logging support' -m 'This adds comprehensive logging to the application'",
            {
                cwd: localRepoDir,
                stdio: "ignore",
            },
        );

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--fill",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
        });

        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain("PR created at");
    });

    it("Should fail when using both --fill and --editor", () => {
        switchBranchAndAddCommit("feature-branch", localRepoDir);

        const result = runGitForge({
            args: [
                "pr",
                "create",
                "--no-browser",
                "--api",
                forge,
                "--api-url",
                getApiUrl(forge),
                "--fill",
                "--editor",
            ],
            cwd: localRepoDir,
            env: { [token]: "test-token" },
            throwsError: true,
        });

        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toContain("cannot be used with");
    });
});

function switchBranchAndAddCommit(newBranch: string, cwd: string) {
    execSync(
        `git checkout -b ${newBranch} && git commit --allow-empty -m 'Some commit'`,
        {
            cwd,
            stdio: "ignore",
        },
    );
}
